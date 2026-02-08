use serde::{Deserialize, Serialize};
use waki::Client;

const SCRAPE_URL: &str = "https://api.firecrawl.dev/v1/scrape";

#[derive(Serialize)]
struct ScrapeRequest<'a> {
    url: &'a str,
    formats: Vec<&'a str>,
}

#[derive(Deserialize)]
struct ScrapeResponse {
    success: bool,
    data: Option<ScrapeData>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ScrapeData {
    markdown: Option<String>,
}

fn api_key() -> Result<String, String> {
    std::env::var("FIRECRAWL_KEY").map_err(|_| "FIRECRAWL_KEY is not set".to_string())
}

pub fn scrape(url: &str) -> Result<String, String> {
    let api_key = api_key()?;
    let request_body = ScrapeRequest {
        url,
        formats: vec!["markdown"],
    };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    let client = Client::new();
    let response = client
        .post(SCRAPE_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    let parsed: ScrapeResponse = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
    if !parsed.success {
        return Err(parsed.error.unwrap_or_else(|| "unknown error".to_string()));
    }
    parsed
        .data
        .and_then(|d| d.markdown)
        .ok_or_else(|| "no markdown content in response".to_string())
}
