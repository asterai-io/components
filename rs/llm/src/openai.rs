use crate::bindings::exports::asterai::llm::llm::{
    ChatMessage, ChatResponse as WitChatResponse, ChatRole, ToolCall as WitToolCall,
    ToolDefinition,
};
use crate::utils::exp_backoff::{retry_with_exp_backoff, RequestOutcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use waki::Client;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct SimpleChatRequest<'a> {
    model: &'a str,
    messages: Vec<SimpleMessage<'a>>,
}

#[derive(Serialize)]
struct SimpleMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct SimpleChatResponse {
    choices: Vec<SimpleChoice>,
}

#[derive(Deserialize)]
struct SimpleChoice {
    message: SimpleResponseMessage,
}

#[derive(Deserialize)]
struct SimpleResponseMessage {
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<MessageBody>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolBody>,
}

#[derive(Serialize)]
struct MessageBody {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCallBody>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
struct ToolCallBody {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCallBody,
}

#[derive(Serialize)]
struct FunctionCallBody {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct ToolBody {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefBody,
}

#[derive(Serialize)]
struct FunctionDefBody {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize)]
struct ChatResponseBody {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ResponseToolCall>,
}

#[derive(Deserialize)]
struct ResponseToolCall {
    id: String,
    function: ResponseFunction,
}

#[derive(Deserialize)]
struct ResponseFunction {
    name: String,
    arguments: String,
}

pub fn prompt(prompt: &str, model: &str) -> String {
    let api_key = match std::env::var("OPENAI_KEY") {
        Ok(key) => key,
        Err(_) => return "error: OPENAI_KEY is not set".to_string(),
    };
    make_request(OPENAI_API_URL, prompt, model, &api_key)
        .unwrap_or_else(|e| format!("error: {e}"))
}

pub fn make_request(
    url: &str,
    prompt: &str,
    model: &str,
    api_key: &str,
) -> Result<String, String> {
    let request_body = SimpleChatRequest {
        model,
        messages: vec![SimpleMessage {
            role: "user",
            content: prompt,
        }],
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    retry_with_exp_backoff(|| send_simple_request(url, api_key, &body_json))
}

fn send_simple_request(
    url: &str,
    api_key: &str,
    body_json: &str,
) -> Result<RequestOutcome, String> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status_code();
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    if status >= 200 && status < 300 {
        let resp: SimpleChatResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
        let content = resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
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
    let api_key = match std::env::var("OPENAI_KEY") {
        Ok(key) => key,
        Err(_) => return error_response("OPENAI_KEY is not set"),
    };
    make_chat_request(OPENAI_API_URL, messages, tools, model, &api_key)
}

pub fn make_chat_request(
    url: &str,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    model: &str,
    api_key: &str,
) -> WitChatResponse {
    let api_messages: Vec<MessageBody> = messages.iter().map(wit_msg_to_openai).collect();
    let api_tools: Vec<ToolBody> = tools
        .iter()
        .map(|t| ToolBody {
            tool_type: "function".to_string(),
            function: FunctionDefBody {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: serde_json::from_str(&t.parameters_json_schema)
                    .unwrap_or(Value::Object(serde_json::Map::new())),
            },
        })
        .collect();
    let request_body = ChatRequest {
        model: model.to_string(),
        messages: api_messages,
        tools: api_tools,
    };
    let body_json = match serde_json::to_string(&request_body) {
        Ok(j) => j,
        Err(e) => return error_response(&format!("failed to serialize: {e}")),
    };
    match retry_with_exp_backoff(|| send_chat_request(url, api_key, &body_json)) {
        Ok(resp) => resp,
        Err(e) => error_response(&format!("{e}")),
    }
}

fn send_chat_request(
    url: &str,
    api_key: &str,
    body_json: &str,
) -> Result<RequestOutcome<WitChatResponse>, String> {
    let client = Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let status = response.status_code();
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    if status >= 200 && status < 300 {
        let resp: ChatResponseBody = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "no response from model".to_string())?;
        let tool_calls: Vec<WitToolCall> = choice
            .message
            .tool_calls
            .into_iter()
            .map(|tc| WitToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments_json: tc.function.arguments,
            })
            .collect();
        return Ok(RequestOutcome::Success(WitChatResponse {
            content: choice.message.content.unwrap_or_default(),
            tool_calls,
        }));
    }
    if status == 429 || status == 403 || status >= 500 {
        return Ok(RequestOutcome::Retryable(status, text));
    }
    Ok(RequestOutcome::Failure(text))
}

fn wit_msg_to_openai(msg: &ChatMessage) -> MessageBody {
    match msg.role {
        ChatRole::Assistant if !msg.tool_calls.is_empty() => MessageBody {
            role: "assistant".to_string(),
            content: if msg.content.is_empty() {
                None
            } else {
                Some(msg.content.clone())
            },
            tool_calls: Some(
                msg.tool_calls
                    .iter()
                    .map(|tc| ToolCallBody {
                        id: tc.id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCallBody {
                            name: tc.name.clone(),
                            arguments: tc.arguments_json.clone(),
                        },
                    })
                    .collect(),
            ),
            tool_call_id: None,
        },
        ChatRole::Tool => MessageBody {
            role: "tool".to_string(),
            content: Some(msg.content.clone()),
            tool_calls: None,
            tool_call_id: msg.tool_call_id.clone(),
        },
        _ => MessageBody {
            role: role_str(msg.role),
            content: Some(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
        },
    }
}

fn role_str(role: ChatRole) -> String {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::Tool => "tool",
    }
    .to_string()
}

pub fn error_response(msg: &str) -> WitChatResponse {
    WitChatResponse {
        content: format!("error: {msg}"),
        tool_calls: Vec::new(),
    }
}
