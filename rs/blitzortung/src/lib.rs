use crate::bindings::asterai::host::api;
use crate::bindings::asterai::host_ws::connection;
use crate::bindings::asterai::host_ws::connection::{Config, ConnectionId};
use crate::bindings::exports::asterai::host_ws::incoming_handler::Guest as WsHandlerGuest;
use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;
use serde::Deserialize;
use std::sync::Mutex;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

struct Component;

const WS_URLS: &[&str] = &[
    "wss://live.lightningmaps.org/",
    "wss://live2.lightningmaps.org/",
];
const HANDLERS_ENV: &str = "BLITZORTUNG_INCOMING_HANDLER_COMPONENTS";
const HANDLER_INTERFACE: &str = "asterai:blitzortung/incoming-handler@0.1.0";
const DEFAULT_CENTER_LAT: f64 = -35.1082;
const DEFAULT_CENTER_LON: f64 = 147.3598;
const DEFAULT_RADIUS_KM: f64 = 50.0;
/// Approximate degrees per km at mid-latitudes for bounding box.
const DEG_PER_KM: f64 = 1.0 / 111.0;

static STATE: Mutex<Option<State>> = Mutex::new(None);

struct State {
    handlers: Vec<String>,
    center_lat: f64,
    center_lon: f64,
    radius_km: f64,
}

impl RunGuest for Component {
    fn run() -> Result<(), ()> {
        let handlers = parse_handlers();
        if handlers.is_empty() {
            println!("blitzortung: no handlers configured");
            return Ok(());
        }
        validate_handlers(&handlers)?;
        let center_lat = parse_env_f64("BLITZORTUNG_CENTER_LAT", DEFAULT_CENTER_LAT);
        let center_lon = parse_env_f64("BLITZORTUNG_CENTER_LON", DEFAULT_CENTER_LON);
        let radius_km = parse_env_f64("BLITZORTUNG_RADIUS_KM", DEFAULT_RADIUS_KM);
        println!(
            "blitzortung: monitoring {radius_km}km radius \
             around ({center_lat}, {center_lon})"
        );
        *STATE.lock().unwrap() = Some(State {
            handlers,
            center_lat,
            center_lon,
            radius_km,
        });
        connect_ws()?;
        Ok(())
    }
}

impl WsHandlerGuest for Component {
    fn on_open(id: ConnectionId) {
        let guard = STATE.lock().unwrap();
        let state = match guard.as_ref() {
            Some(s) => s,
            None => return,
        };
        let margin = state.radius_km * DEG_PER_KM * 2.0;
        let north = state.center_lat + margin;
        let south = state.center_lat - margin;
        let west = state.center_lon - margin;
        let east = state.center_lon + margin;
        let subscribe = serde_json::json!({
            "v": 24,
            "i": {},
            "s": false,
            "x": 0, "w": 0, "tx": 0, "tw": 1,
            "a": 4,
            "z": 8,
            "b": true,
            "l": 1, "t": 1,
            "from_lightningmaps_org": true,
            "p": [north, east, south, west],
            "r": "A",
        });
        if let Err(e) = connection::send(id, subscribe.to_string().as_bytes()) {
            eprintln!("blitzortung: subscribe failed: {e}");
        }
        println!("blitzortung: connected, subscribed to bounds [{south},{west}]-[{north},{east}]");
    }

    fn on_message(_id: ConnectionId, data: Vec<u8>) {
        let text = match String::from_utf8(data) {
            Ok(t) => t,
            Err(_) => return,
        };
        let msg: WsMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => return,
        };
        let strokes = match msg.strokes {
            Some(s) if !s.is_empty() => s,
            _ => return,
        };
        let guard = STATE.lock().unwrap();
        let state = match guard.as_ref() {
            Some(s) => s,
            None => return,
        };
        for raw in &strokes {
            let distance_km = haversine_km(state.center_lat, state.center_lon, raw.lat, raw.lon);
            if distance_km > state.radius_km {
                continue;
            }
            let timestamp_secs = raw.time as f64 / 1_000.0;
            dispatch_strike(state, timestamp_secs, raw.lat, raw.lon, distance_km);
        }
    }

    fn on_close(_id: ConnectionId, _code: u16, _reason: String) {}

    fn on_error(_id: ConnectionId, message: String) {
        eprintln!("blitzortung: ws error: {message}");
    }
}

fn connect_ws() -> Result<(), ()> {
    let headers = vec![(
        "Origin".to_string(),
        "https://www.lightningmaps.org".to_string(),
    )];
    for url in WS_URLS {
        let config = Config {
            url: url.to_string(),
            headers: headers.clone(),
            auto_reconnect: true,
        };
        match connection::connect(&config) {
            Ok(_) => {
                println!("blitzortung: connecting to {url}");
                return Ok(());
            }
            Err(e) => eprintln!("blitzortung: {url} failed: {e}"),
        }
    }
    eprintln!("blitzortung: all servers failed");
    Err(())
}

fn dispatch_strike(state: &State, timestamp_secs: f64, lat: f64, lon: f64, distance_km: f64) {
    let args = serde_json::json!([{
        "timestamp-secs": timestamp_secs,
        "lat": lat,
        "lon": lon,
        "distance-km": distance_km,
    }]);
    let args_str = args.to_string();
    for handler in &state.handlers {
        if let Err(e) =
            api::call_component_function(handler, "incoming-handler/on-strike", &args_str)
        {
            eprintln!(
                "blitzortung: dispatch to {handler} failed ({:?}): {}",
                e.kind, e.message
            );
        }
    }
}

fn parse_handlers() -> Vec<String> {
    std::env::var(HANDLERS_ENV)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn validate_handlers(handlers: &[String]) -> Result<(), ()> {
    for handler in handlers {
        if api::get_component(handler).is_none() {
            eprintln!("blitzortung: {handler} not found in environment");
            return Err(());
        }
        if !api::component_implements(handler, HANDLER_INTERFACE) {
            eprintln!("blitzortung: {handler} does not export {HANDLER_INTERFACE}");
            return Err(());
        }
    }
    Ok(())
}

fn parse_env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let a = (d_lat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}

#[derive(Deserialize)]
struct WsMessage {
    strokes: Option<Vec<RawStrike>>,
}

#[derive(Deserialize)]
struct RawStrike {
    time: u64,
    lat: f64,
    lon: f64,
}

bindings::export!(Component with_types_in bindings);
