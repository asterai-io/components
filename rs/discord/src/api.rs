use crate::bindings::exports::asterai::discord::api::Guest;
use crate::bindings::exports::asterai::discord::types::User;
use crate::Component;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};
use waki::Client;

static TOKEN: LazyLock<String> = LazyLock::new(|| {
    std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN env var is required")
});
static SELF_USER: OnceLock<UserResponse> = OnceLock::new();

impl Guest for Component {
    fn get_self() -> User {
        let cached = SELF_USER.get_or_init(|| fetch_self().expect("failed to fetch bot user"));
        User {
            username: cached.username.clone(),
            id: cached.id.clone(),
        }
    }

    fn send_message(content: String, channel_id: String) -> String {
        send_message_inner(&content, &channel_id).unwrap_or_else(|e| format!("error: {e}"))
    }
}

#[derive(Deserialize, Clone)]
struct UserResponse {
    username: String,
    id: String,
}

#[derive(Deserialize)]
struct MessageResponse {
    id: String,
}

fn token() -> &'static str {
    &TOKEN
}

fn fetch_self() -> Result<UserResponse, String> {
    let token = token();
    let response = Client::new()
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", &format!("Bot {token}"))
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))
}

fn send_message_inner(content: &str, channel_id: &str) -> Result<String, String> {
    let token = token();
    let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");
    let body = serde_json::json!({ "content": content });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let response = Client::new()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bot {token}"))
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let msg: MessageResponse =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    Ok(msg.id)
}
