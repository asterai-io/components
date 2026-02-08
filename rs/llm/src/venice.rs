use crate::openai;

const VENICE_API_URL: &str = "https://api.venice.ai/api/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("VENICE_KEY") {
        Ok(key) => key,
        Err(_) => return "error: VENICE_KEY is not set".to_string(),
    };
    openai::make_request(VENICE_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
