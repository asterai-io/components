use crate::utils::exp_backoff::{retry_with_exp_backoff, RequestOutcome};
use serde::{Deserialize, Serialize};
use waki::Client;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("ANTHROPIC_KEY") {
        Ok(key) => key,
        Err(_) => return "error: ANTHROPIC_KEY is not set".to_string(),
    };
    make_request(prompt, model, &api_key).unwrap_or_else(|e| format!("error: {e}"))
}

fn make_request(prompt: &str, model: &str, api_key: &str) -> Result<String, String> {
    let request_body = MessagesRequest {
        model,
        max_tokens: 4096,
        messages: vec![Message {
            role: "user",
            content: prompt,
        }],
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    retry_with_exp_backoff(|| send_request(api_key, &body_json))
}

fn send_request(api_key: &str, body_json: &str) -> Result<RequestOutcome, String> {
    let client = Client::new();
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status_code();
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    if status >= 200 && status < 300 {
        let resp: MessagesResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
        let content = resp
            .content
            .into_iter()
            .next()
            .map(|b| b.text)
            .ok_or_else(|| "no response from model".to_string())?;
        return Ok(RequestOutcome::Success(content));
    }
    if status == 429 || status == 403 || status >= 500 {
        return Ok(RequestOutcome::Retryable(status, text));
    }
    Ok(RequestOutcome::Failure(text))
}
