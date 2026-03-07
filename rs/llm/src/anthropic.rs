use crate::bindings::exports::asterai::llm::llm::{
    ChatMessage, ChatResponse as WitChatResponse, ChatRole, ToolCall as WitToolCall,
    ToolDefinition,
};
use crate::utils::exp_backoff::{retry_with_exp_backoff, RequestOutcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use waki::Client;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct SimpleMessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<SimpleMessage<'a>>,
}

#[derive(Serialize)]
struct SimpleMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct SimpleMessagesResponse {
    content: Vec<SimpleContentBlock>,
}

#[derive(Deserialize)]
struct SimpleContentBlock {
    text: String,
}

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<MessageBody>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolBody>,
}

#[derive(Serialize)]
struct MessageBody {
    role: String,
    content: MessageContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct ToolBody {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("ANTHROPIC_KEY") {
        Ok(key) => key,
        Err(_) => return "error: ANTHROPIC_KEY is not set".to_string(),
    };
    make_prompt_request(prompt, model, &api_key).unwrap_or_else(|e| format!("error: {e}"))
}

fn make_prompt_request(prompt: &str, model: &str, api_key: &str) -> Result<String, String> {
    let request_body = SimpleMessagesRequest {
        model,
        max_tokens: 4096,
        messages: vec![SimpleMessage {
            role: "user",
            content: prompt,
        }],
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    retry_with_exp_backoff(|| send_prompt_request(api_key, &body_json))
}

fn send_prompt_request(api_key: &str, body_json: &str) -> Result<RequestOutcome, String> {
    let client = Client::new();
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status_code();
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    if status >= 200 && status < 300 {
        let resp: SimpleMessagesResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
        let content = resp
            .content
            .into_iter()
            .next()
            .map(|b| b.text)
            .ok_or_else(|| "no response from model".to_string())?;
        return Ok(RequestOutcome::Success(content));
    }
    if status == 429 || status == 403 || status >= 500 {
        return Ok(RequestOutcome::Retryable(status, text));
    }
    Ok(RequestOutcome::Failure(text))
}

pub fn chat(
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    model: &str,
) -> WitChatResponse {
    let api_key = match std::env::var("ANTHROPIC_KEY") {
        Ok(key) => key,
        Err(_) => return error_response("ANTHROPIC_KEY is not set"),
    };
    make_chat_request(messages, tools, model, &api_key)
}

fn make_chat_request(
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    model: &str,
    api_key: &str,
) -> WitChatResponse {
    // Extract system message (Anthropic uses a top-level `system` field).
    let system = messages
        .iter()
        .find(|m| matches!(m.role, ChatRole::System))
        .map(|m| m.content.clone());
    let api_messages = build_anthropic_messages(&messages);
    let api_tools: Vec<ToolBody> = tools
        .iter()
        .map(|t| ToolBody {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: serde_json::from_str(&t.parameters_json_schema)
                .unwrap_or(Value::Object(serde_json::Map::new())),
        })
        .collect();
    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: 4096,
        system,
        messages: api_messages,
        tools: api_tools,
    };
    let body_json = match serde_json::to_string(&request_body) {
        Ok(j) => j,
        Err(e) => return error_response(&format!("failed to serialize: {e}")),
    };
    match retry_with_exp_backoff(|| send_chat_request(api_key, &body_json)) {
        Ok(resp) => resp,
        Err(e) => error_response(&format!("{e}")),
    }
}

fn send_chat_request(
    api_key: &str,
    body_json: &str,
) -> Result<RequestOutcome<WitChatResponse>, String> {
    let client = Client::new();
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status_code();
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    if status >= 200 && status < 300 {
        let resp: MessagesResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for block in resp.content {
            match block {
                ResponseContentBlock::Text { text } => {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&text);
                }
                ResponseContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(WitToolCall {
                        id,
                        name,
                        arguments_json: serde_json::to_string(&input).unwrap_or_default(),
                    });
                }
            }
        }
        return Ok(RequestOutcome::Success(WitChatResponse {
            content,
            tool_calls,
        }));
    }
    if status == 429 || status == 403 || status >= 500 {
        return Ok(RequestOutcome::Retryable(status, text));
    }
    Ok(RequestOutcome::Failure(text))
}

/// Converts WIT messages to Anthropic's message format.
/// Anthropic requires:
/// - No system role in messages (handled separately)
/// - tool_use blocks inside assistant messages
/// - tool_result blocks inside user messages
/// - Consecutive same-role messages must be merged
fn build_anthropic_messages(messages: &[ChatMessage]) -> Vec<MessageBody> {
    let mut result: Vec<MessageBody> = Vec::new();
    for msg in messages {
        if matches!(msg.role, ChatRole::System) {
            continue;
        }
        match msg.role {
            ChatRole::Assistant if !msg.tool_calls.is_empty() => {
                let mut blocks = Vec::new();
                if !msg.content.is_empty() {
                    blocks.push(ContentBlock::Text {
                        text: msg.content.clone(),
                    });
                }
                for tc in &msg.tool_calls {
                    blocks.push(ContentBlock::ToolUse {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        input: serde_json::from_str(&tc.arguments_json)
                            .unwrap_or(Value::Object(serde_json::Map::new())),
                    });
                }
                result.push(MessageBody {
                    role: "assistant".to_string(),
                    content: MessageContent::Blocks(blocks),
                });
            }
            ChatRole::Tool => {
                let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
                let block = ContentBlock::ToolResult {
                    tool_use_id: tool_call_id,
                    content: msg.content.clone(),
                };
                // Anthropic requires tool_result blocks inside user messages.
                // Merge into preceding user message if possible.
                if let Some(last) = result.last_mut() {
                    if last.role == "user" {
                        match &mut last.content {
                            MessageContent::Blocks(blocks) => {
                                blocks.push(block);
                                continue;
                            }
                            MessageContent::Text(text) => {
                                let text_block = ContentBlock::Text {
                                    text: std::mem::take(text),
                                };
                                last.content =
                                    MessageContent::Blocks(vec![text_block, block]);
                                continue;
                            }
                        }
                    }
                }
                result.push(MessageBody {
                    role: "user".to_string(),
                    content: MessageContent::Blocks(vec![block]),
                });
            }
            _ => {
                result.push(MessageBody {
                    role: role_str(msg.role),
                    content: MessageContent::Text(msg.content.clone()),
                });
            }
        }
    }
    result
}

fn role_str(role: ChatRole) -> String {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "user",
    }
    .to_string()
}

fn error_response(msg: &str) -> WitChatResponse {
    WitChatResponse {
        content: format!("error: {msg}"),
        tool_calls: Vec::new(),
    }
}
