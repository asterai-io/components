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
        parse_tool_name(tool_name).ok_or((-32602, "Invalid params"))?;
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
    let tool_name = format!("{component_name}/{fn_name}");
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

/// Parses "comp-ns:comp-name/interface/function" into
/// ("comp-ns:comp-name", "interface/function").
fn parse_tool_name(tool_name: &str) -> Option<(String, String)> {
    let colon_pos = tool_name.find(':')?;
    let slash_pos = tool_name[colon_pos..].find('/')?;
    let split = colon_pos + slash_pos;
    let component_name = &tool_name[..split];
    let function_name = &tool_name[split + 1..];
    match component_name.is_empty() || function_name.is_empty() {
        true => None,
        false => Some((component_name.to_string(), function_name.to_string())),
    }
}

/// Converts MCP arguments object to the JSON array format expected by
/// call_component_function, preserving parameter order from the function
/// signature.
fn build_args_json(arguments: &Value, func: &FunctionInfo) -> String {
    let args_array: Vec<Value> = func
        .inputs
        .iter()
        .map(|param| arguments.get(&param.name).cloned().unwrap_or(Value::Null))
        .collect();
    serde_json::to_string(&args_array).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_name() {
        let (comp, func) = parse_tool_name("asterai:bom/api/search-locations").unwrap();
        assert_eq!(comp, "asterai:bom");
        assert_eq!(func, "api/search-locations");
    }

    #[test]
    fn test_parse_tool_name_bare() {
        let (comp, func) = parse_tool_name("my:component/do-something").unwrap();
        assert_eq!(comp, "my:component");
        assert_eq!(func, "do-something");
    }

    #[test]
    fn test_parse_tool_name_invalid() {
        assert!(parse_tool_name("no-colon/func").is_none());
        assert!(parse_tool_name("ns:comp").is_none());
        assert!(parse_tool_name("").is_none());
    }

    #[test]
    fn test_is_optional_type() {
        assert!(is_optional_type("option<string>"));
        assert!(!is_optional_type("string"));
        assert!(!is_optional_type("list<string>"));
    }
}
