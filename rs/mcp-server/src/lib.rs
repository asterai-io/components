use crate::bindings::exports::wasi::http::incoming_handler::Guest as HttpGuest;
use crate::bindings::wasi::http::types::{
    Fields, IncomingBody, IncomingRequest, Method, OutgoingBody, OutgoingResponse, ResponseOutparam,
};

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
        match request.method() {
            Method::Post => handle_post(&request, response_out),
            _ => respond(response_out, 405, "application/json", ""),
        }
    }
}

fn handle_post(request: &IncomingRequest, response_out: ResponseOutparam) {
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
        // Notification (no id) — acknowledge with 202.
        (Some("notifications/initialized"), None) => {
            respond(response_out, 202, "application/json", "");
        }
        // Request (has id).
        (Some(method), Some(id)) => {
            let result = mcp::handle_method(method, params);
            let response = match result {
                Ok(value) => mcp::success_response(id, value),
                Err((code, msg)) => mcp::error_response(id, code, msg),
            };
            respond_json(response_out, 200, &response);
        }
        // Malformed.
        _ => {
            let error = mcp::error_response(serde_json::Value::Null, -32600, "Invalid request");
            respond_json(response_out, 200, &error);
        }
    }
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
