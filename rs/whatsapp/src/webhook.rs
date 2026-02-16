use crate::Component;
use crate::bindings::asterai::host::api;
use crate::bindings::exports::wasi::http::incoming_handler::Guest as HttpGuest;
use crate::bindings::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::env;
use std::sync::LazyLock;

const HANDLERS_ENV_NAME: &str = "WHATSAPP_INCOMING_HANDLER_COMPONENTS";
const HANDLER_INTERFACE_NAME: &str = "asterai:whatsapp/incoming-handler@0.1.0";

static VERIFY_TOKEN: LazyLock<String> = LazyLock::new(|| {
    env::var("WHATSAPP_WEBHOOK_VERIFY_TOKEN")
        .expect("WHATSAPP_WEBHOOK_VERIFY_TOKEN env var is required")
});
static APP_SECRET: LazyLock<String> = LazyLock::new(|| {
    env::var("WHATSAPP_APP_SECRET").expect("WHATSAPP_APP_SECRET env var is required")
});

impl HttpGuest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        match request.method() {
            Method::Get => handle_verification(&request, response_out),
            Method::Post => {
                let signature = get_signature(&request);
                let body = read_request_body(&request);
                let Some(body) = body else {
                    respond_status(response_out, 400);
                    return;
                };
                if !verify_signature(&body, signature.as_deref()) {
                    respond_status(response_out, 401);
                    return;
                }
                handle_webhook(&body);
                respond_ok(response_out);
            }
            _ => respond_status(response_out, 405),
        }
    }
}

fn get_signature(request: &IncomingRequest) -> Option<String> {
    let headers = request.headers();
    let values = headers.get(&"x-hub-signature-256".to_string());
    values
        .into_iter()
        .next()
        .and_then(|v| String::from_utf8(v).ok())
}

fn verify_signature(body: &str, signature: Option<&str>) -> bool {
    let Some(signature) = signature else {
        return false;
    };
    let Some(hex_sig) = signature.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(expected) = hex_decode(hex_sig) else {
        return false;
    };
    let mut mac =
        Hmac::<Sha256>::new_from_slice(APP_SECRET.as_bytes()).expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    mac.verify_slice(&expected).is_ok()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

fn handle_verification(request: &IncomingRequest, response_out: ResponseOutparam) {
    let query = request.path_with_query().unwrap_or_default();
    let params = parse_query_params(&query);
    let mode = params
        .iter()
        .find(|(k, _)| k == "hub.mode")
        .map(|(_, v)| v.as_str());
    let token = params
        .iter()
        .find(|(k, _)| k == "hub.verify_token")
        .map(|(_, v)| v.as_str());
    let challenge = params
        .iter()
        .find(|(k, _)| k == "hub.challenge")
        .map(|(_, v)| v.clone());
    if mode != Some("subscribe") || token != Some(&*VERIFY_TOKEN) {
        respond_status(response_out, 403);
        return;
    }
    let challenge = challenge.unwrap_or_default();
    respond_with_body(response_out, 200, &challenge);
}

fn parse_query_params(path_with_query: &str) -> Vec<(String, String)> {
    let query = match path_with_query.split_once('?') {
        Some((_, q)) => q,
        None => return vec![],
    };
    query
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((urldecode(key), urldecode(value)))
        })
        .collect()
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().and_then(|c| (c as char).to_digit(16));
            let lo = bytes.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(hi), Some(lo)) = (hi, lo) {
                result.push((hi * 16 + lo) as u8 as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

#[derive(Deserialize)]
struct WebhookPayload {
    entry: Option<Vec<Entry>>,
}

#[derive(Deserialize)]
struct Entry {
    changes: Option<Vec<Change>>,
}

#[derive(Deserialize)]
struct Change {
    value: Option<ChangeValue>,
}

#[derive(Deserialize)]
struct ChangeValue {
    contacts: Option<Vec<Contact>>,
    messages: Option<Vec<WaMessage>>,
}

#[derive(Deserialize)]
struct Contact {
    profile: Option<Profile>,
    wa_id: Option<String>,
}

#[derive(Deserialize)]
struct Profile {
    name: Option<String>,
}

#[derive(Deserialize)]
struct WaMessage {
    from: String,
    id: String,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    text: Option<TextBody>,
}

#[derive(Deserialize)]
struct TextBody {
    body: String,
}

fn handle_webhook(body: &str) {
    let payload: WebhookPayload = match serde_json::from_str(body) {
        Ok(p) => p,
        Err(e) => return eprintln!("whatsapp: invalid webhook payload: {e}"),
    };
    let Some(entries) = payload.entry else {
        return;
    };
    for entry in entries {
        let Some(changes) = entry.changes else {
            continue;
        };
        for change in changes {
            let Some(value) = change.value else {
                continue;
            };
            let contacts = value.contacts.unwrap_or_default();
            let Some(messages) = value.messages else {
                continue;
            };
            for msg in messages {
                if msg.msg_type.as_deref() != Some("text") {
                    continue;
                }
                let Some(text) = msg.text else {
                    continue;
                };
                let name = contacts
                    .iter()
                    .find(|c| c.wa_id.as_deref() == Some(&msg.from))
                    .and_then(|c| c.profile.as_ref())
                    .and_then(|p| p.name.clone())
                    .unwrap_or_default();
                dispatch_message(&text.body, &name, &msg.from, &msg.id);
            }
        }
    }
}

fn dispatch_message(content: &str, name: &str, phone: &str, message_id: &str) {
    let handlers = parse_handlers();
    if handlers.is_empty() {
        return;
    }
    let args = serde_json::json!([{
        "content": content,
        "sender": {
            "name": name,
            "phone": phone,
        },
        "id": message_id,
    }]);
    let args_str = args.to_string();
    for handler in handlers {
        match api::call_component_function(&handler, "incoming-handler/on-message", &args_str) {
            Ok(_) => {}
            Err(e) => eprintln!(
                "whatsapp: dispatch to {handler} failed ({:?}): {}",
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

fn respond_ok(response_out: ResponseOutparam) {
    respond_status(response_out, 200);
}

fn respond_status(response_out: ResponseOutparam, status: u16) {
    let headers = Fields::new();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(status).unwrap();
    let body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    OutgoingBody::finish(body, None).unwrap();
}

fn respond_with_body(response_out: ResponseOutparam, status: u16, content: &str) {
    let headers = Fields::new();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(status).unwrap();
    let body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    let stream = body.write().unwrap();
    stream.blocking_write_and_flush(content.as_bytes()).unwrap();
    drop(stream);
    OutgoingBody::finish(body, None).unwrap();
}
