use serde::{Deserialize, Serialize};
use waki::Client;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("OPENAI_KEY") {
        Ok(key) => key,
        Err(_) => return "error: OPENAI_KEY is not set".to_string(),
    };
    make_request(prompt, model, &api_key).unwrap_or_else(|e| format!("error: {e}"))
}

fn make_request(prompt: &str, model: &str, api_key: &str) -> Result<String, String> {
    let request_body = ChatRequest {
        model,
        messages: vec![Message {
            role: "user",
            content: prompt,
        }],
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    let client = Client::new();
    let response = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    let chat_response: ChatResponse = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
    chat_response
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "no response from model".to_string())
}
