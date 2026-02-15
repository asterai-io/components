use crate::Component;
use crate::bindings::asterai::host::api;
use crate::bindings::exports::wasi::http::incoming_handler::Guest as HttpGuest;
use crate::bindings::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use serde::Deserialize;
use std::env;
use std::sync::LazyLock;

const HANDLERS_ENV_NAME: &str = "TELEGRAM_INCOMING_HANDLER_COMPONENTS";
const HANDLER_INTERFACE_NAME: &str = "asterai:telegram/incoming-handler@0.1.0";

static WEBHOOK_SECRET: LazyLock<Option<String>> = LazyLock::new(|| {
    env::var("TELEGRAM_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
});

#[derive(Deserialize)]
struct SetWebhookResponse {
    ok: bool,
    description: Option<String>,
}

impl HttpGuest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        if !verify_secret(&request) {
            respond_unauthorized(response_out);
            return;
        }
        let body = read_request_body(&request);
        if let Some(body) = body {
            handle_update(&body);
        }
        respond_ok(response_out);
    }
}

fn verify_secret(request: &IncomingRequest) -> bool {
    let Some(expected_secret) = WEBHOOK_SECRET.as_deref() else {
        return false;
    };
    let headers = request.headers();
    let values = headers.get("x-telegram-bot-api-secret-token");
    values.iter().any(|v| v.as_slice() == expected_secret.as_bytes())
}

fn read_request_body(request: &IncomingRequest) -> Option<String> {
    let incoming_body = request.consume().ok()?;
    let stream = incoming_body.stream().ok()?;
    let mut buf = Vec::new();
    loop {
        match stream.read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                buf.extend_from_slice(&chunk);
            }
            Err(_) => break,
        }
    }
    drop(stream);
    IncomingBody::finish(incoming_body);
    String::from_utf8(buf).ok()
}

fn respond_unauthorized(response_out: ResponseOutparam) {
    let headers = Fields::new();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(401).unwrap();
    let body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    OutgoingBody::finish(body, None).unwrap();
}

fn respond_ok(response_out: ResponseOutparam) {
    let headers = Fields::new();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(200).unwrap();
    let body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    OutgoingBody::finish(body, None).unwrap();
}

#[derive(Deserialize)]
struct Update {
    message: Option<MessageData>,
}

#[derive(Deserialize)]
struct MessageData {
    message_id: i64,
    text: Option<String>,
    from: Option<FromData>,
    chat: ChatData,
}

#[derive(Deserialize)]
struct FromData {
    id: i64,
    username: Option<String>,
    first_name: String,
}

#[derive(Deserialize)]
struct ChatData {
    id: i64,
}

fn handle_update(body: &str) {
    let update: Update = match serde_json::from_str(body) {
        Ok(u) => u,
        Err(e) => return eprintln!("telegram: invalid update: {e}"),
    };
    let Some(msg) = update.message else {
        return;
    };
    let Some(text) = msg.text else {
        return;
    };
    let from = msg.from.as_ref();
    let username = from
        .and_then(|f| f.username.clone())
        .unwrap_or_else(|| from.map(|f| f.first_name.clone()).unwrap_or_default());
    let user_id = from.map(|f| f.id).unwrap_or(0);
    dispatch_message(&text, &username, user_id, msg.message_id, msg.chat.id);
}

fn dispatch_message(content: &str, username: &str, user_id: i64, message_id: i64, chat_id: i64) {
    let handlers = parse_handlers();
    if handlers.is_empty() {
        return;
    }
    let args = serde_json::json!([{
        "content": content,
        "sender": {
            "username": username,
            "id": user_id,
        },
        "id": message_id,
        "chat-id": chat_id,
    }]);
    let args_str = args.to_string();
    for handler in handlers {
        match api::call_component_function(&handler, "incoming-handler/on-message", &args_str) {
            Ok(_) => {}
            Err(e) => eprintln!(
                "telegram: dispatch to {handler} failed ({:?}): {}",
                e.kind, e.message
            ),
        }
    }
}

pub fn parse_handlers() -> Vec<String> {
    let raw = env::var(HANDLERS_ENV_NAME).unwrap_or_default();
    raw.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn validate_handlers(handlers: &[String]) -> Result<(), ()> {
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

pub fn setup_webhook() -> Result<(), ()> {
    let webhook_url = env::var("TELEGRAM_WEBHOOK_URL")
        .map_err(|_| eprintln!("missing TELEGRAM_WEBHOOK_URL env var"))?;
    let webhook_secret = env::var("TELEGRAM_WEBHOOK_SECRET")
        .map_err(|_| eprintln!("missing TELEGRAM_WEBHOOK_SECRET env var"))?;
    let token = crate::api::token();
    let url = format!("https://api.telegram.org/bot{token}/setWebhook");
    let body = serde_json::json!({
        "url": webhook_url,
        "secret_token": webhook_secret,
    });
    let body_str = serde_json::to_string(&body).map_err(|e| eprintln!("json error: {e}"))?;
    let response = waki::Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| eprintln!("setWebhook request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| eprintln!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| eprintln!("invalid utf8: {e}"))?;
    let resp: SetWebhookResponse = serde_json::from_str(&text)
        .map_err(|e| eprintln!("parse setWebhook response failed: {e}: {text}"))?;
    if !resp.ok {
        eprintln!(
            "setWebhook failed: {}",
            resp.description.unwrap_or_else(|| "unknown error".into())
        );
        return Err(());
    }
    Ok(())
}
