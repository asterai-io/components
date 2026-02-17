use crate::bindings::exports::asterai::bom::api::Guest;
use crate::bindings::exports::asterai::bom::types::{DailyForecast, Location, Warning};
use serde::Deserialize;
use waki::Client;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

struct Component;

const API_BASE: &str = "https://api.weather.bom.gov.au/v1";

impl Guest for Component {
    fn search_locations(query: String) -> Vec<Location> {
        match fetch_locations(&query) {
            Ok(locations) => locations,
            Err(e) => {
                eprintln!("bom: search-locations failed: {e}");
                Vec::new()
            }
        }
    }

    fn get_daily_forecasts(geohash: String) -> Vec<DailyForecast> {
        match fetch_daily_forecasts(&geohash) {
            Ok(forecasts) => forecasts,
            Err(e) => {
                eprintln!("bom: get-daily-forecasts failed: {e}");
                Vec::new()
            }
        }
    }

    fn get_warnings(geohash: String) -> Vec<Warning> {
        match fetch_warnings(&geohash) {
            Ok(warnings) => warnings,
            Err(e) => {
                eprintln!("bom: get-warnings failed: {e}");
                Vec::new()
            }
        }
    }
}

fn fetch_locations(query: &str) -> Result<Vec<Location>, String> {
    let url = format!("{API_BASE}/locations?search={}", urlencode(query));
    let resp: BomResponse<Vec<BomLocation>> = fetch_json(&url)?;
    Ok(resp
        .data
        .into_iter()
        .map(|l| Location {
            geohash: l.geohash,
            name: l.name,
            state: l.state.unwrap_or_default(),
        })
        .collect())
}

fn fetch_daily_forecasts(geohash: &str) -> Result<Vec<DailyForecast>, String> {
    let url = format!("{API_BASE}/locations/{geohash}/forecasts/daily");
    let resp: BomResponse<Vec<BomForecastDay>> = fetch_json(&url)?;
    Ok(resp
        .data
        .into_iter()
        .map(|f| DailyForecast {
            date: f.date,
            icon_descriptor: f.icon_descriptor.unwrap_or_default(),
            short_text: f.short_text.unwrap_or_default(),
            extended_text: f.extended_text.unwrap_or_default(),
            temp_min: f.temp_min,
            temp_max: f.temp_max,
            rain_chance: f.rain.and_then(|r| r.chance),
        })
        .collect())
}

fn fetch_warnings(geohash: &str) -> Result<Vec<Warning>, String> {
    let url = format!("{API_BASE}/locations/{geohash}/warnings");
    let resp: BomResponse<Vec<BomWarning>> = fetch_json(&url)?;
    Ok(resp
        .data
        .into_iter()
        .map(|w| Warning {
            id: w.id,
            short_title: w.short_title.unwrap_or_default(),
            warning_type: w.warning_type.unwrap_or_default(),
            title: w.title.unwrap_or_default(),
        })
        .collect())
}

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let response = Client::new()
        .get(url)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("parse failed: {e}"))
}

fn urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => result.push_str(&format!("%{byte:02X}")),
        }
    }
    result
}

#[derive(Deserialize)]
struct BomResponse<T> {
    data: T,
}

#[derive(Deserialize)]
struct BomLocation {
    geohash: String,
    name: String,
    state: Option<String>,
}

#[derive(Deserialize)]
struct BomForecastDay {
    date: String,
    icon_descriptor: Option<String>,
    short_text: Option<String>,
    extended_text: Option<String>,
    temp_min: Option<f64>,
    temp_max: Option<f64>,
    rain: Option<BomRain>,
}

#[derive(Deserialize)]
struct BomRain {
    chance: Option<u32>,
}

#[derive(Deserialize)]
struct BomWarning {
    id: String,
    short_title: Option<String>,
    #[serde(rename = "type")]
    warning_type: Option<String>,
    title: Option<String>,
}

bindings::export!(Component with_types_in bindings);
