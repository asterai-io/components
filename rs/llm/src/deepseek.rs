use crate::openai;

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("DEEPSEEK_KEY") {
        Ok(key) => key,
        Err(_) => return "error: DEEPSEEK_KEY is not set".to_string(),
    };
    openai::make_request(DEEPSEEK_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
