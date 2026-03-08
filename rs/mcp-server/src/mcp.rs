use crate::bindings::asterai::host::api;
use crate::bindings::asterai::host::api::{ComponentInfo, FunctionInfo};
use serde_json::{Value, json};
use std::env;
use std::sync::LazyLock;

const SELF_COMPONENT: &str = "asterai:mcp-server";
const SERVER_NAME: &str = "asterai-mcp-server";
const SERVER_VERSION: &str = "0.1.0";
const PROTOCOL_VERSION: &str = "2025-03-26";
const TOOLS_ENV: &str = "MCP_SERVER_TOOLS";
const SKIP_INTERFACES: &[&str] = &["run", "incoming-handler"];

static ALLOWED_COMPONENTS: LazyLock<Option<Vec<String>>> = LazyLock::new(|| {
    let raw = env::var(TOOLS_ENV).ok()?;
    let items: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    match items.is_empty() {
        true => None,
        false => Some(items),
    }
});

pub fn handle_method(method: &str, params: Option<Value>) -> Result<Value, (i32, &'static str)> {
    match method {
        "initialize" => handle_initialize(),
        "ping" => Ok(json!({})),
        "tools/list" => handle_tools_list(params),
        "tools/call" => handle_tools_call(params),
        _ => Err((-32601, "Method not found")),
    }
}

pub fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub fn error_response(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

fn handle_initialize() -> Result<Value, (i32, &'static str)> {
    Ok(json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {},
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION,
        },
    }))
}

fn handle_tools_list(_params: Option<Value>) -> Result<Value, (i32, &'static str)> {
    eprintln!("mcp-server: tools/list called");
    let components = list_allowed_components();
    eprintln!("mcp-server: got {} components", components.len());
    let mut tools = Vec::new();
    for comp in &components {
        eprintln!(
            "mcp-server: component {} has {} functions",
            comp.name,
            comp.functions.len()
        );
        for func in &comp.functions {
            if is_skip_function(func) {
                continue;
            }
            tools.push(function_to_tool(&comp.name, func));
        }
    }
    eprintln!("mcp-server: returning {} tools", tools.len());
    Ok(json!({ "tools": tools }))
}

fn handle_tools_call(params: Option<Value>) -> Result<Value, (i32, &'static str)> {
    let params = params.ok_or((-32602, "Invalid params"))?;
    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((-32602, "Invalid params"))?;
    eprintln!("mcp-server: tools/call {tool_name}");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    let (component_name, function_name) =
        decode_tool_name(tool_name).ok_or((-32602, "Invalid params"))?;
    if !is_component_allowed(&component_name) {
        return Err((-32602, "Invalid params"));
    }
    let comp = api::get_component(&component_name).ok_or((-32602, "Invalid params"))?;
    let func = comp
        .functions
        .iter()
        .find(|f| format_function_name(f) == function_name)
        .ok_or((-32602, "Invalid params"))?;
    let args_json = build_args_json(&arguments, func);
    match api::call_component_function(&component_name, &function_name, &args_json) {
        Ok(result) => Ok(json!({
            "content": [{ "type": "text", "text": result }],
            "isError": false,
        })),
        Err(e) => Ok(json!({
            "content": [{ "type": "text", "text": e.message }],
            "isError": true,
        })),
    }
}

fn list_allowed_components() -> Vec<ComponentInfo> {
    let all = api::list_other_components();
    let filtered: Vec<ComponentInfo> = match ALLOWED_COMPONENTS.as_ref() {
        Some(allowed) => all
            .into_iter()
            .filter(|c| allowed.iter().any(|a| a == &c.name))
            .collect(),
        None => all,
    };
    // Filter out self — list_other_components should exclude it but
    // doesn't always work in HTTP handler contexts.
    filtered
        .into_iter()
        .filter(|c| c.name != SELF_COMPONENT)
        .collect()
}

fn is_component_allowed(name: &str) -> bool {
    if name == SELF_COMPONENT {
        return false;
    }
    match ALLOWED_COMPONENTS.as_ref() {
        Some(allowed) => allowed.iter().any(|a| a == name),
        None => api::list_other_components().iter().any(|c| c.name == name),
    }
}

fn is_skip_function(func: &FunctionInfo) -> bool {
    let Some(iface) = &func.interface_name else {
        return false;
    };
    SKIP_INTERFACES.contains(&iface.as_str())
}

fn function_to_tool(component_name: &str, func: &FunctionInfo) -> Value {
    let fn_name = format_function_name(func);
    let tool_name = encode_tool_name(component_name, &fn_name);
    let description = func
        .description
        .clone()
        .unwrap_or_else(|| format!("{component_name} {fn_name}"));
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for param in &func.inputs {
        let schema: Value =
            serde_json::from_str(&param.type_schema).unwrap_or(json!({ "type": "string" }));
        properties.insert(param.name.clone(), schema);
        if !is_optional_type(&param.type_name) {
            required.push(Value::String(param.name.clone()));
        }
    }
    json!({
        "name": tool_name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        },
    })
}

fn is_optional_type(type_name: &str) -> bool {
    type_name.starts_with("option<")
}

fn format_function_name(func: &FunctionInfo) -> String {
    match &func.interface_name {
        Some(iface) => format!("{iface}/{}", func.name),
        None => func.name.clone(),
    }
}

/// Encodes a tool name for MCP using only `[a-zA-Z0-9_-]`.
/// `:` → `--`, `/` → `_`.
/// e.g. ("asterai:cli", "common/ls") → "asterai--cli_common_ls"
fn encode_tool_name(component_name: &str, function_name: &str) -> String {
    let comp = component_name.replace(':', "--");
    let func = function_name.replace('/', "_");
    format!("{comp}_{func}")
}

/// Decodes an MCP tool name back to (component_name, function_name).
/// e.g. "asterai--cli_common_ls" → ("asterai:cli", "common/ls")
fn decode_tool_name(tool_name: &str) -> Option<(String, String)> {
    let dash_pos = tool_name.find("--")?;
    let sep_pos = dash_pos + 2 + tool_name[dash_pos + 2..].find('_')?;
    let comp_encoded = &tool_name[..sep_pos];
    let func_encoded = &tool_name[sep_pos + 1..];
    if comp_encoded.is_empty() || func_encoded.is_empty() {
        return None;
    }
    let component_name = comp_encoded.replace("--", ":");
    let function_name = func_encoded.replace('_', "/");
    Some((component_name, function_name))
}

/// Converts MCP arguments object to the JSON array format expected by
/// call_component_function, preserving parameter order from the function
/// signature.
fn build_args_json(arguments: &Value, func: &FunctionInfo) -> String {
    let args_array: Vec<Value> = func
        .inputs
        .iter()
        .map(|param| {
            let value = arguments.get(&param.name).cloned().unwrap_or(Value::Null);
            coerce_arg(value, &param.type_name)
        })
        .collect();
    serde_json::to_string(&args_array).unwrap_or_else(|_| "[]".to_string())
}

/// Coerces an MCP argument value to match the expected WIT type.
/// Handles string→bytes conversion for list<u8> params.
fn coerce_arg(value: Value, type_name: &str) -> Value {
    if !is_bytes_type(type_name) {
        return value;
    }
    // Convert string to byte array for list<u8> params.
    match value {
        Value::String(s) => Value::Array(s.bytes().map(|b| Value::Number(b.into())).collect()),
        other => other,
    }
}

fn is_bytes_type(type_name: &str) -> bool {
    type_name == "list<u8>" || type_name == "option<list<u8>>"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_tool_name() {
        assert_eq!(
            encode_tool_name("asterai:bom", "api/search-locations"),
            "asterai--bom_api_search-locations"
        );
        assert_eq!(
            encode_tool_name("asterai:cli", "common/ls"),
            "asterai--cli_common_ls"
        );
    }

    #[test]
    fn test_decode_tool_name() {
        let (comp, func) = decode_tool_name("asterai--bom_api_search-locations").unwrap();
        assert_eq!(comp, "asterai:bom");
        assert_eq!(func, "api/search-locations");
    }

    #[test]
    fn test_decode_tool_name_bare() {
        let (comp, func) = decode_tool_name("my--component_do-something").unwrap();
        assert_eq!(comp, "my:component");
        assert_eq!(func, "do-something");
    }

    #[test]
    fn test_roundtrip() {
        let cases = vec![
            ("asterai:cli", "common/ls"),
            ("asterai:fs-local", "fs/read"),
            ("my:comp", "bare-func"),
        ];
        for (comp, func) in cases {
            let encoded = encode_tool_name(comp, func);
            let (dec_comp, dec_func) = decode_tool_name(&encoded).unwrap();
            assert_eq!(dec_comp, comp);
            assert_eq!(dec_func, func);
        }
    }

    #[test]
    fn test_decode_invalid() {
        assert!(decode_tool_name("no-double-dash").is_none());
        assert!(decode_tool_name("ns--comp").is_none());
        assert!(decode_tool_name("").is_none());
    }

    #[test]
    fn test_is_optional_type() {
        assert!(is_optional_type("option<string>"));
        assert!(!is_optional_type("string"));
        assert!(!is_optional_type("list<string>"));
    }
}
