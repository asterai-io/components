use crate::bindings::asterai::blitzortung::types::Strike;
use crate::bindings::asterai::bom::api as bom;
use crate::bindings::asterai::bom::types::{DailyForecast, Warning};
use crate::bindings::asterai::host_cron::scheduler;
use crate::bindings::exports::asterai::blitzortung::incoming_handler::Guest as StrikeHandlerGuest;
use crate::bindings::exports::asterai::lightning_notifier::jobs::Guest as JobsGuest;
use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

mod llm;
mod notify;
mod state;

pub struct Component;

const COMPONENT_NAME: &str = "asterai:lightning-notifier";
const SHOULD_SEND_DAILY_BOM_FORECAST: bool = true;

impl RunGuest for Component {
    fn run() -> Result<(), ()> {
        notify::validate_env().map_err(|e| eprintln!("lightning-notifier: {e}"))?;
        schedule_cron("0 */5 * * * *", "jobs/check-warnings");
        schedule_cron("0 0 23 * * *", "jobs/check-forecast");
        println!("lightning-notifier: ready, running initial checks");
        <Component as JobsGuest>::check_warnings();
        <Component as JobsGuest>::check_forecast();
        Ok(())
    }
}

impl StrikeHandlerGuest for Component {
    fn on_strike(strike: Strike) {
        let host_dir = match state::resolve_host_dir() {
            Ok(d) => d,
            Err(e) => return eprintln!("lightning-notifier: {e}"),
        };
        let mut st = state::load(&host_dir);
        st.strike_count += 1;
        let now = now_secs();
        let is_cooldown =
            now.saturating_sub(st.last_strike_notify_secs) < state::STRIKE_COOLDOWN_SECS;
        if !is_cooldown && !is_quiet_hours() {
            let msg = format!(
                "Lightning detected {:.0}km from Wagga Wagga! \
                 ({} strikes since last alert)",
                strike.distance_km, st.strike_count
            );
            notify::send(&msg);
            st.last_strike_notify_secs = now;
            st.strike_count = 0;
        }
        state::save(&host_dir, &st);
    }
}

impl JobsGuest for Component {
    fn check_warnings() {
        let host_dir = match state::resolve_host_dir() {
            Ok(d) => d,
            Err(e) => return eprintln!("lightning-notifier: {e}"),
        };
        let mut st = state::load(&host_dir);
        let geohash = resolve_geohash(&host_dir, &mut st);
        let warnings = bom::get_warnings(&geohash);
        let new_warnings = find_new_thunderstorm_warnings(&warnings, &st.notified_warning_ids);
        for (id, title) in &new_warnings {
            if !is_quiet_hours() {
                notify::send(&format!("BOM Warning: {title}"));
            }
            st.notified_warning_ids.push(id.clone());
        }
        if !new_warnings.is_empty() {
            state::save(&host_dir, &st);
        }
    }

    fn check_forecast() {
        let host_dir = match state::resolve_host_dir() {
            Ok(d) => d,
            Err(e) => return eprintln!("lightning-notifier: {e}"),
        };
        let mut st = state::load(&host_dir);
        let now = now_secs();
        let is_cooldown =
            now.saturating_sub(st.last_forecast_notify_secs) < state::FORECAST_COOLDOWN_SECS;
        if is_cooldown {
            return;
        }
        let geohash = resolve_geohash(&host_dir, &mut st);
        let forecasts = bom::get_daily_forecasts(&geohash);
        let has_storm = is_storm_forecast(&forecasts);
        if SHOULD_SEND_DAILY_BOM_FORECAST {
            let msg = format_daily_forecast(&forecasts);
            notify::send(&msg);
            st.last_forecast_notify_secs = now;
            state::save(&host_dir, &st);
        } else if has_storm {
            notify::send(
                "BOM forecasts possible thunderstorms \
                 for Wagga Wagga today.",
            );
            st.last_forecast_notify_secs = now;
            state::save(&host_dir, &st);
        }
    }
}

fn schedule_cron(cron: &str, function: &str) {
    match scheduler::create_schedule(cron, COMPONENT_NAME, function, "[]") {
        Ok(id) => println!("lightning-notifier: scheduled {function} (id {id})"),
        Err(e) => eprintln!("lightning-notifier: failed to schedule {function}: {e}"),
    }
}

fn resolve_geohash(host_dir: &str, st: &mut state::NotifierState) -> String {
    if !st.geohash.is_empty() {
        return st.geohash.clone();
    }
    let locations = bom::search_locations("Wagga Wagga");
    let geohash = match locations.first() {
        Some(loc) => loc.geohash.clone(),
        None => {
            eprintln!("lightning-notifier: could not resolve geohash for Wagga Wagga");
            return String::new();
        }
    };
    println!("lightning-notifier: resolved geohash: {geohash}");
    st.geohash = geohash.clone();
    state::save(host_dir, st);
    geohash
}

fn find_new_thunderstorm_warnings(
    warnings: &[Warning],
    known_ids: &[String],
) -> Vec<(String, String)> {
    warnings
        .iter()
        .filter(|w| {
            w.warning_type.contains("thunderstorm")
                || w.title.to_lowercase().contains("thunderstorm")
        })
        .filter(|w| !known_ids.contains(&w.id))
        .map(|w| (w.id.clone(), w.title.clone()))
        .collect()
}

fn format_daily_forecast(forecasts: &[DailyForecast]) -> String {
    let Some(today) = forecasts.first() else {
        return "Wagga Wagga forecast: no data available.".to_string();
    };
    let temp = match (today.temp_min, today.temp_max) {
        (Some(min), Some(max)) => format!("{min:.0}-{max:.0}C"),
        (None, Some(max)) => format!("up to {max:.0}C"),
        (Some(min), None) => format!("from {min:.0}C"),
        (None, None) => "temp N/A".to_string(),
    };
    let rain = match today.rain_chance {
        Some(c) => format!("{c}% rain"),
        None => "rain N/A".to_string(),
    };
    let text = match today.short_text.is_empty() {
        true => &today.extended_text,
        false => &today.short_text,
    };
    format!("Wagga Wagga forecast: {text}. {temp}, {rain}.")
}

fn is_storm_forecast(forecasts: &[DailyForecast]) -> bool {
    let Some(today) = forecasts.first() else {
        return false;
    };
    today.icon_descriptor == "storm" || today.extended_text.to_lowercase().contains("thunderstorm")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns true if current UTC hour is between 13:00-19:59 (11 PM - 6 AM AEST).
fn is_quiet_hours() -> bool {
    let secs = now_secs();
    let utc_hour = (secs % 86400) / 3600;
    (13..20).contains(&utc_hour)
}

bindings::export!(Component with_types_in bindings);
