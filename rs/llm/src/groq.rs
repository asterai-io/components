use crate::openai;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("GROQ_KEY") {
        Ok(key) => key,
        Err(_) => return "error: GROQ_KEY is not set".to_string(),
    };
    openai::make_request(GROQ_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
