use crate::openai;

const FIREWORKS_API_URL: &str = "https://api.fireworks.ai/inference/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("FIREWORKS_KEY") {
        Ok(key) => key,
        Err(_) => return "error: FIREWORKS_KEY is not set".to_string(),
    };
    openai::make_request(FIREWORKS_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
