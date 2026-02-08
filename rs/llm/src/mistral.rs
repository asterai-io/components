use crate::openai;

const MISTRAL_API_URL: &str = "https://api.mistral.ai/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("MISTRAL_KEY") {
        Ok(key) => key,
        Err(_) => return "error: MISTRAL_KEY is not set".to_string(),
    };
    openai::make_request(MISTRAL_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
