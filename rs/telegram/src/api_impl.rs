use crate::bindings::exports::asterai::telegram::api::Guest;
use crate::bindings::exports::asterai::telegram::types::User;
use crate::Component;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};
use waki::Client;

const API_BASE: &str = "https://api.telegram.org/bot";

static TOKEN: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN env var is required")
});
static SELF_USER: OnceLock<UserResponse> = OnceLock::new();

impl Guest for Component {
    fn get_self() -> User {
        let cached = SELF_USER.get_or_init(|| fetch_self().expect("failed to fetch bot user"));
        User {
            username: cached.username.clone(),
            id: cached.id,
        }
    }

    fn send_message(content: String, chat_id: i64) -> String {
        send_message_inner(&content, chat_id).unwrap_or_else(|e| format!("error: {e}"))
    }
}

#[derive(Deserialize, Clone)]
struct UserResponse {
    username: String,
    id: i64,
}

#[derive(Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct MessageResponse {
    message_id: i64,
}

pub fn token() -> &'static str {
    &TOKEN
}

fn api_url(method: &str) -> String {
    format!("{}{}/{}", API_BASE, token(), method)
}

fn fetch_self() -> Result<UserResponse, String> {
    let response = Client::new()
        .get(&api_url("getMe"))
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let resp: TelegramResponse<UserResponse> =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    if !resp.ok {
        return Err(resp.description.unwrap_or_else(|| "unknown error".into()));
    }
    resp.result.ok_or_else(|| "missing result".into())
}

fn send_message_inner(content: &str, chat_id: i64) -> Result<String, String> {
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": content,
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let response = Client::new()
        .post(&api_url("sendMessage"))
        .header("Content-Type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let resp: TelegramResponse<MessageResponse> =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    if !resp.ok {
        return Err(resp.description.unwrap_or_else(|| "unknown error".into()));
    }
    let msg = resp.result.ok_or_else(|| "missing result".to_string())?;
    Ok(msg.message_id.to_string())
}
