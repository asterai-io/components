use crate::bindings::asterai::host::api;
use crate::bindings::asterai::host_ws::connection;
use crate::bindings::asterai::host_ws::connection::Config;
use crate::bindings::exports::asterai::host_ws::incoming_handler::{
    ConnectionId, Guest as IncomingHandlerGuest,
};
use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;
use serde::Deserialize;
use std::env;
use std::sync::Mutex;
use crate::gateway_opcode::GatewayOpcode;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}
mod gateway_opcode;

struct Component;

struct State {
    targets: Vec<String>,
    token: String,
    sequence: Option<u64>,
    session_id: Option<String>,
    heartbeat_interval_ms: u64,
    messages_since_heartbeat: u32,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

// GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT
const INTENTS: u64 = (1 << 9) | (1 << 12) | (1 << 15);
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const HEARTBEAT_MSG_THRESHOLD: u32 = 5;

#[derive(Deserialize)]
struct GatewayPayload {
    op: GatewayOpcode,
    d: Option<serde_json::Value>,
    s: Option<u64>,
    t: Option<String>,
}

#[derive(Deserialize)]
struct HelloData {
    heartbeat_interval: u64,
}

#[derive(Deserialize)]
struct MessageData {
    content: String,
    author: UserData,
    id: String,
    channel_id: String,
}

#[derive(Deserialize)]
struct UserData {
    username: String,
    id: String,
}

#[derive(Deserialize)]
struct ReadyData {
    session_id: String,
}

impl RunGuest for Component {
    fn run() -> Result<(), ()> {
        let targets = parse_targets()?;
        validate_targets(&targets)?;
        let token =
            env::var("DISCORD_TOKEN").map_err(|_| eprintln!("missing DISCORD_TOKEN env var"))?;
        let config = Config {
            url: GATEWAY_URL.to_string(),
            headers: vec![],
            auto_reconnect: true,
        };
        *STATE.lock().unwrap() = Some(State {
            targets,
            token,
            sequence: None,
            session_id: None,
            heartbeat_interval_ms: 0,
            messages_since_heartbeat: 0,
        });
        connection::connect(&config).map_err(|e| eprintln!("connection failed: {e}"))?;
        Ok(())
    }
}

impl IncomingHandlerGuest for Component {
    fn on_message(id: ConnectionId, data: Vec<u8>) {
        let text = match String::from_utf8(data) {
            Ok(t) => t,
            Err(e) => return eprintln!("invalid utf8: {e}"),
        };
        let payload: GatewayPayload = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => return eprintln!("invalid gateway payload: {e}"),
        };
        let mut guard = STATE.lock().unwrap();
        let state = match guard.as_mut() {
            Some(s) => s,
            None => return eprintln!("state not initialized"),
        };
        if let Some(s) = payload.s {
            state.sequence = Some(s);
        }
        match payload.op {
            GatewayOpcode::Hello => handle_hello(id, state, payload.d),
            GatewayOpcode::HeartbeatAck => {}
            GatewayOpcode::Heartbeat => send_heartbeat(id, state),
            GatewayOpcode::Dispatch => {
                handle_dispatch(state, payload.t, payload.d);
                maybe_heartbeat(id, state);
            }
            GatewayOpcode::Reconnect => connection::close(id),
            GatewayOpcode::InvalidSession => {
                state.session_id = None;
                state.sequence = None;
                connection::close(id);
            }
            _ => {}
        }
    }

    fn on_close(_id: ConnectionId, _code: u16, _reason: String) {}

    fn on_error(_id: ConnectionId, message: String) {
        eprintln!("ws error: {message}");
    }
}

fn parse_targets() -> Result<Vec<String>, ()> {
    let raw = env::var("DISCORD_LISTENER_TARGETS")
        .map_err(|_| eprintln!("missing DISCORD_LISTENER_TARGETS env var"))?;
    let targets: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if targets.is_empty() {
        eprintln!("DISCORD_LISTENER_TARGETS is empty");
        return Err(());
    }
    Ok(targets)
}

fn validate_targets(targets: &[String]) -> Result<(), ()> {
    for target in targets {
        let is_valid = api::component_implements(
            target,
            "asterai:discord-message-listener/incoming-handler@0.1.0",
        );
        if !is_valid {
            eprintln!("{target} does not implement incoming-handler interface");
            return Err(());
        }
    }
    Ok(())
}

fn handle_hello(id: ConnectionId, state: &mut State, data: Option<serde_json::Value>) {
    let Some(data) = data else {
        return eprintln!("hello missing data");
    };
    let hello: HelloData = match serde_json::from_value(data) {
        Ok(h) => h,
        Err(e) => return eprintln!("invalid hello data: {e}"),
    };
    state.heartbeat_interval_ms = hello.heartbeat_interval;
    send_heartbeat(id, state);
    match (&state.session_id, state.sequence) {
        (Some(session_id), Some(seq)) => {
            let resume = serde_json::json!({
                "op": GatewayOpcode::Resume,
                "d": {
                    "token": state.token,
                    "session_id": session_id,
                    "seq": seq,
                }
            });
            send_json(id, &resume);
        }
        _ => {
            let identify = serde_json::json!({
                "op": GatewayOpcode::Identify,
                "d": {
                    "token": state.token,
                    "intents": INTENTS,
                    "properties": {
                        "os": "linux",
                        "browser": "asterai",
                        "device": "asterai"
                    }
                }
            });
            send_json(id, &identify);
        }
    }
}

fn handle_dispatch(state: &mut State, event: Option<String>, data: Option<serde_json::Value>) {
    let (Some(event), Some(data)) = (event, data) else {
        return;
    };
    match event.as_str() {
        "READY" => {
            if let Ok(ready) = serde_json::from_value::<ReadyData>(data) {
                state.session_id = Some(ready.session_id);
            }
        }
        "MESSAGE_CREATE" => {
            let msg: MessageData = match serde_json::from_value(data) {
                Ok(m) => m,
                Err(e) => return eprintln!("invalid message data: {e}"),
            };
            dispatch_message(state, &msg);
        }
        _ => {}
    }
}

fn dispatch_message(state: &State, msg: &MessageData) {
    let args = serde_json::json!([{
        "content": msg.content,
        "author": {
            "username": msg.author.username,
            "id": msg.author.id
        },
        "id": msg.id,
        "channel-id": msg.channel_id
    }]);
    let args_str = args.to_string();
    for target in &state.targets {
        if let Err(e) =
            api::call_component_function(target, "incoming-handler/on-message", &args_str)
        {
            eprintln!("dispatch to {target} failed ({:?}): {}", e.kind, e.message);
        }
    }
}

fn send_heartbeat(id: ConnectionId, state: &mut State) {
    let payload = serde_json::json!({
        "op": GatewayOpcode::Heartbeat,
        "d": state.sequence
    });
    send_json(id, &payload);
    state.messages_since_heartbeat = 0;
}

fn maybe_heartbeat(id: ConnectionId, state: &mut State) {
    state.messages_since_heartbeat += 1;
    if state.messages_since_heartbeat >= HEARTBEAT_MSG_THRESHOLD {
        send_heartbeat(id, state);
    }
}

fn send_json(id: ConnectionId, value: &serde_json::Value) {
    let json = match serde_json::to_string(value) {
        Ok(j) => j,
        Err(e) => return eprintln!("json serialization failed: {e}"),
    };
    if let Err(e) = connection::send(id, json.as_bytes()) {
        eprintln!("ws send failed: {e}");
    }
}

bindings::export!(Component with_types_in bindings);
