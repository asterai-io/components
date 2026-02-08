use crate::openai;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("OPENROUTER_KEY") {
        Ok(key) => key,
        Err(_) => return "error: OPENROUTER_KEY is not set".to_string(),
    };
    openai::make_request(OPENROUTER_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
