use crate::openai;

const TOGETHER_API_URL: &str = "https://api.together.xyz/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("TOGETHER_KEY") {
        Ok(key) => key,
        Err(_) => return "error: TOGETHER_KEY is not set".to_string(),
    };
    openai::make_request(TOGETHER_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
