use serde::{Deserialize, Serialize};

const STATE_FILENAME: &str = "lightning-notifier-state.json";

/// 4 hours in seconds.
pub const STRIKE_COOLDOWN_SECS: u64 = 4 * 60 * 60;

/// 24 hours in seconds.
pub const FORECAST_COOLDOWN_SECS: u64 = 24 * 60 * 60;

#[derive(Serialize, Deserialize, Default)]
pub struct NotifierState {
    pub last_strike_notify_secs: u64,
    pub strike_count: u32,
    pub last_forecast_notify_secs: u64,
    pub notified_warning_ids: Vec<String>,
    pub geohash: String,
}

pub fn load(host_dir: &str) -> NotifierState {
    let path = format!("{host_dir}/{STATE_FILENAME}");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(host_dir: &str, state: &NotifierState) {
    let _ = std::fs::create_dir_all(host_dir);
    let path = format!("{host_dir}/{STATE_FILENAME}");
    let json = match serde_json::to_string_pretty(state) {
        Ok(j) => j,
        Err(e) => return eprintln!("lightning-notifier: failed to serialize state: {e}"),
    };
    if let Err(e) = std::fs::write(&path, json) {
        eprintln!("lightning-notifier: failed to write state: {e}");
    }
}

pub fn resolve_host_dir() -> Result<String, String> {
    if let Ok(dirs) = std::env::var("ASTERAI_ALLOWED_DIRS") {
        if let Some(first) = dirs.split(':').next() {
            if !first.is_empty() {
                return Ok(first.to_string());
            }
        }
    }
    Err("no host directory available \u{2014} pass --allow-dir".to_string())
}
