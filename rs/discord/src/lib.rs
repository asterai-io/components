use crate::bindings::exports::asterai::discord::discord::Guest;
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

impl Guest for Component {
    fn send_message(content: String, channel_id: String) -> String {
        send_message_inner(&content, &channel_id).unwrap_or_else(|e| format!("error: {e}"))
    }
}

#[derive(Deserialize)]
struct MessageResponse {
    id: String,
}

fn send_message_inner(content: &str, channel_id: &str) -> Result<String, String> {
    let token =
        std::env::var("DISCORD_TOKEN").map_err(|_| "DISCORD_TOKEN env var is required")?;
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

bindings::export!(Component with_types_in bindings);
