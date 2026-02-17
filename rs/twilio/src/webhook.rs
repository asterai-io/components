use crate::Component;
use crate::bindings::asterai::host::api;
use crate::bindings::exports::wasi::http::incoming_handler::Guest as HttpGuest;
use crate::bindings::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, OutgoingBody, OutgoingResponse, ResponseOutparam,
};
use base64::prelude::*;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha1::Sha1;
use std::env;
use std::sync::LazyLock;
use waki::Client;

const HANDLERS_ENV_NAME: &str = "TWILIO_INCOMING_HANDLER_COMPONENTS";
const HANDLER_INTERFACE_NAME: &str = "asterai:twilio/incoming-handler@0.1.0";

static WEBHOOK_URL: LazyLock<Option<String>> =
    LazyLock::new(|| env::var("TWILIO_WEBHOOK_URL").ok());

impl HttpGuest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let body = match read_request_body(&request) {
            Some(b) => b,
            None => return respond_status(response_out, 400),
        };
        if !verify_signature(&request, &body) {
            respond_status(response_out, 401);
            return;
        }
        handle_sms(&body);
        respond_ok(response_out);
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
    let webhook_url = env::var("TWILIO_WEBHOOK_URL")
        .map_err(|_| eprintln!("missing TWILIO_WEBHOOK_URL env var"))?;
    let pn_sid = crate::api::fetch_phone_number_sid()
        .map_err(|e| eprintln!("failed to find phone number: {e}"))?;
    update_sms_url(&pn_sid, &webhook_url)
        .map_err(|e| eprintln!("failed to set webhook URL: {e}"))?;
    Ok(())
}

fn verify_signature(request: &IncomingRequest, body: &str) -> bool {
    let Some(url) = WEBHOOK_URL.as_deref() else {
        return false;
    };
    let headers = request.headers();
    let sig_values = headers.get("x-twilio-signature");
    let Some(sig_bytes) = sig_values.into_iter().next() else {
        return false;
    };
    let Ok(expected) = String::from_utf8(sig_bytes) else {
        return false;
    };
    // Sort POST parameters alphabetically and append to URL.
    let mut params = parse_form_body(body);
    params.sort_by(|a, b| a.0.cmp(&b.0));
    let mut data = url.to_string();
    for (key, value) in &params {
        data.push_str(key);
        data.push_str(value);
    }
    // HMAC-SHA1 with auth token.
    let Ok(mut mac) = Hmac::<Sha1>::new_from_slice(crate::api::auth_token().as_bytes()) else {
        return false;
    };
    mac.update(data.as_bytes());
    let result = mac.finalize().into_bytes();
    BASE64_STANDARD.encode(&result) == expected
}

fn handle_sms(body: &str) {
    let params = parse_form_body(body);
    let get = |key: &str| {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    let content = get("Body");
    let from = get("From");
    let message_sid = get("MessageSid");
    if content.is_empty() || from.is_empty() {
        return;
    }
    dispatch_message(content, from, message_sid);
}

fn dispatch_message(content: &str, from: &str, message_id: &str) {
    let handlers = parse_handlers();
    if handlers.is_empty() {
        return;
    }
    let args = serde_json::json!([{
        "content": content,
        "sender": {
            "name": "",
            "phone": from,
        },
        "id": message_id,
    }]);
    let args_str = args.to_string();
    for handler in handlers {
        match api::call_component_function(&handler, "incoming-handler/on-message", &args_str) {
            Ok(_) => {}
            Err(e) => eprintln!(
                "twilio: dispatch to {handler} failed ({:?}): {}",
                e.kind, e.message
            ),
        }
    }
}

fn update_sms_url(pn_sid: &str, webhook_url: &str) -> Result<(), String> {
    let url = crate::api::api_url(&format!("IncomingPhoneNumbers/{pn_sid}.json"));
    let body = format!("SmsUrl={}", crate::api::urlencode(webhook_url));
    let response = Client::new()
        .post(&url)
        .header("Authorization", &crate::api::auth_header())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    if let Ok(err) = serde_json::from_str::<ErrorResponse>(&text) {
        if let Some(code) = err.code {
            let msg = err.message.unwrap_or_else(|| "unknown error".into());
            return Err(format!("error {code}: {msg}"));
        }
    }
    Ok(())
}

fn parse_form_body(body: &str) -> Vec<(String, String)> {
    body.split('&')
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
        match b {
            b'%' => {
                let hi = bytes.next().and_then(|c| (c as char).to_digit(16));
                let lo = bytes.next().and_then(|c| (c as char).to_digit(16));
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    result.push((hi * 16 + lo) as u8 as char);
                }
            }
            b'+' => result.push(' '),
            _ => result.push(b as char),
        }
    }
    result
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

#[derive(Deserialize)]
struct ErrorResponse {
    code: Option<u32>,
    message: Option<String>,
}
