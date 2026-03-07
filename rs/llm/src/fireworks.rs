use crate::bindings::exports::asterai::llm::llm::{
    ChatMessage, ChatResponse as WitChatResponse, ToolDefinition,
};
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

pub fn chat(
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    model: &str,
) -> WitChatResponse {
    let api_key = match std::env::var("FIREWORKS_KEY") {
        Ok(key) => key,
        Err(_) => return openai::error_response("FIREWORKS_KEY is not set"),
    };
    openai::make_chat_request(FIREWORKS_API_URL, messages, tools, model, &api_key)
}
