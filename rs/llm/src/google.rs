use crate::bindings::exports::asterai::llm::llm::{
    ChatMessage, ChatResponse as WitChatResponse, ToolDefinition,
};
use crate::openai;

const GOOGLE_API_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("GOOGLE_KEY") {
        Ok(key) => key,
        Err(_) => return "error: GOOGLE_KEY is not set".to_string(),
    };
    openai::make_request(GOOGLE_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}

pub fn chat(
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    model: &str,
) -> WitChatResponse {
    let api_key = match std::env::var("GOOGLE_KEY") {
        Ok(key) => key,
        Err(_) => return openai::error_response("GOOGLE_KEY is not set"),
    };
    openai::make_chat_request(GOOGLE_API_URL, messages, tools, model, &api_key)
}
