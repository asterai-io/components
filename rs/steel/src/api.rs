use crate::bindings::exports::asterai::steel::types::{
    PdfOptions, ScrapeOptions, ScreenshotOptions, SessionOptions,
};
use serde::Serialize;
use waki::Client;

const BASE_URL: &str = "https://api.steel.dev/v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    use_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solve_captcha: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_ads: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headless: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    screenshot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pdf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    full_page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PdfRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_proxy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: Option<u64>,
}

pub fn create_session(options: &SessionOptions) -> Result<String, String> {
    let req = CreateSessionRequest {
        use_proxy: options.use_proxy,
        solve_captcha: options.solve_captcha,
        block_ads: options.block_ads,
        timeout: options.timeout,
        user_agent: options.user_agent.clone(),
        proxy_url: options.proxy_url.clone(),
        session_id: options.session_id.clone(),
        headless: options.headless,
    };
    let body = serde_json::to_string(&req)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    post("/sessions", body.as_bytes())
}

pub fn release_session(session_id: &str) -> Result<String, String> {
    post(&format!("/sessions/{session_id}/release"), b"{}")
}

pub fn release_all_sessions() -> Result<String, String> {
    post("/sessions/release", b"{}")
}

pub fn get_session(session_id: &str) -> Result<String, String> {
    get(&format!("/sessions/{session_id}"))
}

pub fn list_sessions() -> Result<String, String> {
    get("/sessions")
}

pub fn scrape(options: &ScrapeOptions) -> Result<String, String> {
    let req = ScrapeRequest {
        url: options.url.clone(),
        format: options.formats.clone(),
        screenshot: options.screenshot,
        pdf: options.pdf,
        use_proxy: options.use_proxy,
        delay: options.delay,
    };
    let body = serde_json::to_string(&req)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    post("/scrape", body.as_bytes())
}

pub fn screenshot(options: &ScreenshotOptions) -> Result<String, String> {
    let req = ScreenshotRequest {
        url: options.url.clone(),
        full_page: options.full_page,
        use_proxy: options.use_proxy,
        delay: options.delay,
    };
    let body = serde_json::to_string(&req)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    post("/screenshot", body.as_bytes())
}

pub fn pdf(options: &PdfOptions) -> Result<String, String> {
    let req = PdfRequest {
        url: options.url.clone(),
        use_proxy: options.use_proxy,
        delay: options.delay,
    };
    let body = serde_json::to_string(&req)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    post("/pdf", body.as_bytes())
}

fn api_key() -> Result<String, String> {
    std::env::var("STEEL_API_KEY").map_err(|_| "STEEL_API_KEY is not set".to_string())
}

fn post(path: &str, body: &[u8]) -> Result<String, String> {
    let api_key = api_key()?;
    let url = format!("{BASE_URL}{path}");
    let client = Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("steel-api-key", &api_key)
        .body(body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))
}

fn get(path: &str) -> Result<String, String> {
    let api_key = api_key()?;
    let url = format!("{BASE_URL}{path}");
    let client = Client::new();
    let response = client
        .get(&url)
        .header("steel-api-key", &api_key)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))
}
