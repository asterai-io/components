use crate::bindings::asterai::host::api;
use crate::bindings::asterai::host_ws::connection;
use crate::bindings::asterai::host_ws::connection::{Config, ConnectionId};
use crate::bindings::exports::asterai::host_ws::incoming_handler::Guest as IncomingHandlerGuest;
use crate::listener::gateway_opcode::GatewayOpcode;
use crate::Component;
use serde::Deserialize;
use std::env;
use std::sync::Mutex;

mod gateway_opcode;

static STATE: Mutex<Option<State>> = Mutex::new(None);

const HANDLERS_ENV_NAME: &str = "DISCORD_INCOMING_HANDLER_COMPONENTS";
const HANDLER_INTERFACE_NAME: &str = "asterai:discord/incoming-handler@0.1.0";
// GUILD_MESSAGES | DIRECT_MESSAGES | MESSAGE_CONTENT
const INTENTS: u64 = (1 << 9) | (1 << 12) | (1 << 15);
const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const HEARTBEAT_MSG_THRESHOLD: u32 = 5;

struct State {
    /// Components that implement `incoming-handler` and are
    /// subscribed (via the DISCORD_INCOMING_HANDLER_COMPONENTS env).
    handlers: Vec<String>,
    token: String,
    sequence: Option<u64>,
    session_id: Option<String>,
    heartbeat_interval_ms: u64,
    messages_since_heartbeat: u32,
}

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

pub fn initialise_ws_client() -> Result<(), ()> {
    let handlers = parse_handlers()?;
    if handlers.is_empty() {
        // There are no handlers, so there's nothing to do.
        // This returns before opening the client WS connection,
        // so the listener part of the component ends here and
        // will do nothing.
        return Ok(());
    }
    validate_handlers(&handlers)?;
    let token =
        env::var("DISCORD_TOKEN").map_err(|_| eprintln!("missing DISCORD_TOKEN env var"))?;
    let config = Config {
        url: GATEWAY_URL.to_string(),
        headers: vec![],
        auto_reconnect: true,
    };
    *STATE.lock().unwrap() = Some(State {
        handlers,
        token,
        sequence: None,
        session_id: None,
        heartbeat_interval_ms: 0,
        messages_since_heartbeat: 0,
    });
    connection::connect(&config).map_err(|e| eprintln!("connection failed: {e}"))?;
    Ok(())
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
            GatewayOpcode::Reconnect => {}
            GatewayOpcode::InvalidSession => {
                state.session_id = None;
                state.sequence = None;
                send_identify(id, state);
            }
            _ => {}
        }
    }

    fn on_open(_id: ConnectionId) { }

    fn on_close(_id: ConnectionId, _code: u16, _reason: String) {}

    fn on_error(_id: ConnectionId, message: String) {
        eprintln!("ws error: {message}");
    }
}

fn parse_handlers() -> Result<Vec<String>, ()> {
    let raw = env::var(HANDLERS_ENV_NAME).unwrap_or_default();
    let handlers: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if handlers.is_empty() {
        eprintln!("{HANDLERS_ENV_NAME} is empty");
        return Err(());
    }
    Ok(handlers)
}

fn validate_handlers(handlers: &[String]) -> Result<(), ()> {
    for handler in handlers {
        let component = api::get_component(handler);
        if component.is_none() {
            eprintln!("{handler} not found in environment");
            return Err(());
        }
        let has_interface = api::component_implements(handler, HANDLER_INTERFACE_NAME);
        if !has_interface {
            eprintln!("{handler} does not export {HANDLER_INTERFACE_NAME}");
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
        _ => send_identify(id, state),
    }
}

fn send_identify(id: ConnectionId, state: &State) {
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
    for handler in &state.handlers {
        if let Err(e) =
            api::call_component_function(handler, "incoming-handler/on-message", &args_str)
        {
            eprintln!("dispatch to {handler} failed ({:?}): {}", e.kind, e.message);
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
