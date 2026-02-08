use serde::{Deserialize, Serialize};
use waki::Client;

const SEARCH_URL: &str = "https://api.firecrawl.dev/v1/search";

#[derive(Serialize)]
struct SearchRequest<'a> {
    query: &'a str,
    limit: u32,
}

#[derive(Deserialize)]
struct SearchResponse {
    success: bool,
    data: Option<Vec<SearchResult>>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SearchResult {
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

fn api_key() -> Result<String, String> {
    std::env::var("FIRECRAWL_KEY").map_err(|_| "FIRECRAWL_KEY is not set".to_string())
}

pub fn search(query: &str, limit: u32) -> Result<String, String> {
    let api_key = api_key()?;
    let request_body = SearchRequest { query, limit };
    let body_json =
        serde_json::to_string(&request_body).map_err(|e| format!("failed to serialize: {e}"))?;
    let client = Client::new();
    let response = client
        .post(SEARCH_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {api_key}"))
        .body(body_json.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let body = response
        .body()
        .map_err(|e| format!("failed to read response: {e}"))?;
    let text = String::from_utf8(body).map_err(|e| format!("invalid response encoding: {e}"))?;
    let parsed: SearchResponse = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse response: {e}: {text}"))?;
    if !parsed.success {
        return Err(parsed.error.unwrap_or_else(|| "unknown error".to_string()));
    }
    let results = parsed.data.unwrap_or_default();
    serde_json::to_string(&results).map_err(|e| format!("failed to serialize results: {e}"))
}
