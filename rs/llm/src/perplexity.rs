use crate::openai;

const PERPLEXITY_API_URL: &str = "https://api.perplexity.ai/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("PERPLEXITY_KEY") {
        Ok(key) => key,
        Err(_) => return "error: PERPLEXITY_KEY is not set".to_string(),
    };
    openai::make_request(PERPLEXITY_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}
