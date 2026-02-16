use crate::Component;
use crate::bindings::exports::asterai::whatsapp::api::Guest;
use crate::bindings::exports::asterai::whatsapp::types::User;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};
use waki::Client;

const API_BASE: &str = "https://graph.facebook.com/v21.0";

static ACCESS_TOKEN: LazyLock<String> = LazyLock::new(|| {
    std::env::var("WHATSAPP_ACCESS_TOKEN").expect("WHATSAPP_ACCESS_TOKEN env var is required")
});
static PHONE_NUMBER_ID: LazyLock<String> = LazyLock::new(|| {
    std::env::var("WHATSAPP_PHONE_NUMBER_ID").expect("WHATSAPP_PHONE_NUMBER_ID env var is required")
});
static SELF_USER: OnceLock<PhoneNumberResponse> = OnceLock::new();

impl Guest for Component {
    fn get_self() -> User {
        let cached = SELF_USER
            .get_or_init(|| fetch_self().expect("failed to fetch WhatsApp phone number info"));
        let name = cached
            .verified_name
            .clone()
            .or_else(|| cached.name.clone())
            .unwrap_or_default();
        let phone = cached
            .display_phone_number
            .clone()
            .unwrap_or_else(|| cached.id.clone());
        User { name, phone }
    }

    fn send_message(content: String, to: String) -> String {
        send_message(&content, &to).unwrap_or_else(|e| format!("error: {e}"))
    }
}

#[derive(Deserialize, Clone)]
struct PhoneNumberResponse {
    verified_name: Option<String>,
    name: Option<String>,
    display_phone_number: Option<String>,
    id: String,
}

#[derive(Deserialize)]
struct SendMessageResponse {
    messages: Option<Vec<MessageId>>,
    error: Option<GraphError>,
}

#[derive(Deserialize)]
struct MessageId {
    id: String,
}

#[derive(Deserialize)]
struct GraphError {
    message: String,
}

pub fn access_token() -> &'static str {
    &ACCESS_TOKEN
}

pub fn phone_number_id() -> &'static str {
    &PHONE_NUMBER_ID
}

fn api_url(path: &str) -> String {
    format!("{API_BASE}/{path}")
}

fn auth_header() -> String {
    format!("Bearer {}", access_token())
}

fn fetch_self() -> Result<PhoneNumberResponse, String> {
    let url = api_url(phone_number_id());
    let response = Client::new()
        .get(&url)
        .header("Authorization", &auth_header())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))
}

fn send_message(content: &str, to: &str) -> Result<String, String> {
    let url = api_url(&format!("{}/messages", phone_number_id()));
    let body = serde_json::json!({
        "messaging_product": "whatsapp",
        "to": to,
        "type": "text",
        "text": { "body": content },
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let response = Client::new()
        .post(&url)
        .header("Authorization", &auth_header())
        .header("Content-Type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let resp: SendMessageResponse =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    if let Some(err) = resp.error {
        return Err(err.message);
    }
    let msg = resp
        .messages
        .and_then(|msgs| msgs.into_iter().next())
        .ok_or_else(|| "no message ID in response".to_string())?;
    Ok(msg.id)
}
