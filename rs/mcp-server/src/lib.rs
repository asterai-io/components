use crate::bindings::exports::wasi::http::incoming_handler::Guest as HttpGuest;
use crate::bindings::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

mod auth;
mod mcp;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

struct Component;

impl HttpGuest for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let full_path = request.path_with_query().unwrap_or_default();
        let path = full_path.split('?').next().unwrap_or("");
        let is_token = path == "/token" || path.ends_with("/token");
        let is_authorize = path == "/authorize" || path.ends_with("/authorize");
        eprintln!(
            "mcp-server: {} {}",
            match request.method() {
                Method::Get => "GET",
                Method::Post => "POST",
                _ => "OTHER",
            },
            path,
        );
        match (request.method(), is_authorize, is_token) {
            (Method::Get, true, _) => handle_authorize(&full_path, response_out),
            (Method::Post, _, true) => handle_token(&request, response_out),
            (Method::Post, _, false) => handle_mcp(&request, response_out),
            _ => respond(response_out, 405, "application/json", ""),
        }
    }
}

fn handle_authorize(full_path: &str, response_out: ResponseOutparam) {
    if !auth::is_auth_enabled() {
        respond(response_out, 400, "text/plain", "Auth not enabled");
        return;
    }
    let query = full_path.split_once('?').map(|(_, q)| q).unwrap_or("");
    let params = parse_query(query);
    let get = |key: &str| -> Option<&str> {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let Some(redirect_uri) = get("redirect_uri") else {
        respond(response_out, 400, "text/plain", "Missing redirect_uri");
        return;
    };
    let Some(code_challenge) = get("code_challenge") else {
        respond(response_out, 400, "text/plain", "Missing code_challenge");
        return;
    };
    let state = get("state").unwrap_or("");
    let Some(code) = auth::generate_auth_code(code_challenge) else {
        respond(response_out, 500, "text/plain", "Failed to generate code");
        return;
    };
    let location = if state.is_empty() {
        format!("{redirect_uri}?code={code}")
    } else {
        format!("{redirect_uri}?code={code}&state={state}")
    };
    eprintln!("mcp-server: authorize redirecting to {location}");
    redirect(response_out, &location);
}

fn handle_token(request: &IncomingRequest, response_out: ResponseOutparam) {
    if !auth::is_auth_enabled() {
        respond_json(
            response_out,
            400,
            &serde_json::json!({"error": "invalid_request", "error_description": "Auth not enabled"}),
        );
        return;
    }
    let Some(body) = read_body(request) else {
        respond_json(
            response_out,
            400,
            &serde_json::json!({"error": "invalid_request"}),
        );
        return;
    };
    eprintln!("mcp-server: token request body={body}");
    let params = parse_form_body(&body);
    let get = |key: &str| -> Option<&str> {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    let grant_type = get("grant_type").unwrap_or("");
    match grant_type {
        "authorization_code" => {
            let code = get("code").unwrap_or("");
            let code_verifier = get("code_verifier").unwrap_or("");
            let client_secret = get("client_secret");
            if code.is_empty() || code_verifier.is_empty() {
                respond_json(
                    response_out,
                    400,
                    &serde_json::json!({"error": "invalid_request", "error_description": "Missing code or code_verifier"}),
                );
                return;
            }
            if !auth::verify_auth_code(code, code_verifier, client_secret) {
                eprintln!("mcp-server: auth code verification failed");
                respond_json(
                    response_out,
                    401,
                    &serde_json::json!({"error": "invalid_grant"}),
                );
                return;
            }
            let token = auth::access_token().unwrap();
            eprintln!("mcp-server: token issued");
            respond_json(
                response_out,
                200,
                &serde_json::json!({
                    "access_token": token,
                    "token_type": "Bearer",
                }),
            );
        }
        _ => {
            respond_json(
                response_out,
                400,
                &serde_json::json!({"error": "unsupported_grant_type"}),
            );
        }
    }
}

fn handle_mcp(request: &IncomingRequest, response_out: ResponseOutparam) {
    if auth::is_auth_enabled() {
        let headers = request.headers();
        let auth_values = headers.get(&"authorization".to_string());
        let authorized = auth_values
            .iter()
            .filter_map(|v| std::str::from_utf8(v).ok())
            .any(|v| auth::verify_bearer(v));
        if !authorized {
            eprintln!("mcp-server: 401 unauthorized");
            respond(response_out, 401, "application/json", "");
            return;
        }
    }
    let Some(body) = read_body(request) else {
        eprintln!("mcp-server: failed to read request body");
        respond(response_out, 400, "application/json", "");
        return;
    };
    eprintln!("mcp-server: received: {body}");
    let rpc_request: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            let error = mcp::error_response(serde_json::Value::Null, -32700, "Parse error");
            respond_json(response_out, 200, &error);
            return;
        }
    };
    let method = rpc_request.get("method").and_then(|m| m.as_str());
    let id = rpc_request.get("id").cloned();
    let params = rpc_request.get("params").cloned();
    match (method, id) {
        (Some("notifications/initialized"), None) => {
            respond(response_out, 202, "application/json", "");
        }
        (Some(method), Some(id)) => {
            let result = mcp::handle_method(method, params);
            let response = match result {
                Ok(value) => mcp::success_response(id, value),
                Err((code, msg)) => mcp::error_response(id, code, msg),
            };
            respond_json(response_out, 200, &response);
        }
        _ => {
            let error = mcp::error_response(serde_json::Value::Null, -32600, "Invalid request");
            respond_json(response_out, 200, &error);
        }
    }
}

fn parse_form_body(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((urldecode(k), urldecode(v)))
        })
        .collect()
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    parse_form_body(query)
}

fn urldecode(s: &str) -> String {
    let s = s.replace('+', " ");
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn read_body(request: &IncomingRequest) -> Option<String> {
    let incoming_body = request.consume().ok()?;
    let stream = incoming_body.stream().ok()?;
    let mut buf = Vec::new();
    loop {
        match stream.read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                buf.extend_from_slice(&chunk);
            }
            Err(_) => break,
        }
    }
    drop(stream);
    IncomingBody::finish(incoming_body);
    String::from_utf8(buf).ok()
}

fn respond_json(response_out: ResponseOutparam, status: u16, value: &serde_json::Value) {
    let body = serde_json::to_string(value).unwrap_or_default();
    respond(response_out, status, "application/json", &body);
}

fn redirect(response_out: ResponseOutparam, location: &str) {
    let headers = Fields::new();
    headers
        .set(&"location".to_string(), &[location.as_bytes().to_vec()])
        .ok();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(302).unwrap();
    let out_body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    OutgoingBody::finish(out_body, None).unwrap();
}

fn respond(response_out: ResponseOutparam, status: u16, content_type: &str, body: &str) {
    let headers = Fields::new();
    headers
        .set(
            &"content-type".to_string(),
            &[content_type.as_bytes().to_vec()],
        )
        .ok();
    let response = OutgoingResponse::new(headers);
    response.set_status_code(status).unwrap();
    let out_body = response.body().unwrap();
    ResponseOutparam::set(response_out, Ok(response));
    if !body.is_empty() {
        let stream = out_body.write().unwrap();
        let bytes = body.as_bytes();
        let mut offset = 0;
        while offset < bytes.len() {
            let end = (offset + 4096).min(bytes.len());
            stream
                .blocking_write_and_flush(&bytes[offset..end])
                .unwrap();
            offset = end;
        }
        drop(stream);
    }
    OutgoingBody::finish(out_body, None).unwrap();
}

bindings::export!(Component with_types_in bindings);
