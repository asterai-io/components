use crate::sigv4;
use crate::config::CONFIG;
use std::collections::BTreeMap;
use waki::Client;

pub struct HeadResponse {
    pub content_length: u64,
    pub last_modified: Option<u64>,
}

pub struct S3Object {
    pub key: String,
    pub size: u64,
}

pub struct ListResult {
    pub objects: Vec<S3Object>,
    pub common_prefixes: Vec<String>,
}

// ---------------------------------------------------------------------------
// S3 operations
// ---------------------------------------------------------------------------

pub fn get_object(key: &str) -> Result<Vec<u8>, String> {
    let resp = s3_request("GET", key, "", &[], None, None)?;
    let status = resp.status_code();
    let body = resp.body().map_err(|e| e.to_string())?;
    if status != 200 {
        return Err(error_message(status, &body));
    }
    Ok(body)
}

pub fn get_object_range(key: &str, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    let end = offset + length - 1;
    let range_val = format!("bytes={offset}-{end}");
    let resp = s3_request("GET", key, "", &[], Some(&range_val), None)?;
    let status = resp.status_code();
    let body = resp.body().map_err(|e| e.to_string())?;
    if status != 206 && status != 200 {
        return Err(error_message(status, &body));
    }
    Ok(body)
}

pub fn put_object(key: &str, data: &[u8]) -> Result<(), String> {
    let resp = s3_request("PUT", key, "", data, None, None)?;
    let status = resp.status_code();
    if status != 200 {
        let body = resp.body().map_err(|e| e.to_string())?;
        return Err(error_message(status, &body));
    }
    Ok(())
}

pub fn delete_object(key: &str) -> Result<(), String> {
    let resp = s3_request("DELETE", key, "", &[], None, None)?;
    let status = resp.status_code();
    if status != 204 && status != 200 {
        let body = resp.body().map_err(|e| e.to_string())?;
        return Err(error_message(status, &body));
    }
    Ok(())
}

pub fn head_object(key: &str) -> Result<HeadResponse, String> {
    let resp = s3_request("HEAD", key, "", &[], None, None)?;
    let status = resp.status_code();
    if status != 200 {
        return Err(format!("HEAD returned {status}"));
    }
    let content_length = resp
        .header("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let last_modified = resp
        .header("last-modified")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_http_date);
    Ok(HeadResponse {
        content_length,
        last_modified,
    })
}

pub fn copy_object(src_key: &str, dst_key: &str) -> Result<(), String> {
    let c = &*CONFIG;
    let copy_source = format!("/{}/{}", c.bucket, sigv4::uri_encode(src_key, false));
    let resp = s3_request("PUT", dst_key, "", &[], None, Some(&copy_source))?;
    let status = resp.status_code();
    if status != 200 {
        let body = resp.body().map_err(|e| e.to_string())?;
        return Err(error_message(status, &body));
    }
    Ok(())
}

pub fn list_objects(prefix: &str, delimiter: Option<&str>) -> Result<ListResult, String> {
    let mut all_objects = Vec::new();
    let mut all_prefixes = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut query = format!("list-type=2&prefix={}", sigv4::uri_encode(prefix, true));
        if let Some(d) = delimiter {
            query.push_str(&format!("&delimiter={}", sigv4::uri_encode(d, true)));
        }
        if let Some(ref token) = continuation_token {
            query.push_str(&format!(
                "&continuation-token={}",
                sigv4::uri_encode(token, true)
            ));
        }

        let resp = s3_request("GET", "", &query, &[], None, None)?;
        let status = resp.status_code();
        let body = resp.body().map_err(|e| e.to_string())?;
        if status != 200 {
            return Err(error_message(status, &body));
        }
        let xml = String::from_utf8(body).map_err(|e| e.to_string())?;

        for block in xml_blocks(&xml, "Contents") {
            let key = xml_tag_value(block, "Key").unwrap_or_default().to_string();
            let size: u64 = xml_tag_value(block, "Size")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            all_objects.push(S3Object { key, size });
        }
        for block in xml_blocks(&xml, "CommonPrefixes") {
            if let Some(p) = xml_tag_value(block, "Prefix") {
                all_prefixes.push(p.to_string());
            }
        }

        let truncated = xml_tag_value(&xml, "IsTruncated")
            .map(|v| v == "true")
            .unwrap_or(false);
        if !truncated {
            break;
        }
        continuation_token = xml_tag_value(&xml, "NextContinuationToken").map(|s| s.to_string());
        if continuation_token.is_none() {
            break;
        }
    }

    Ok(ListResult {
        objects: all_objects,
        common_prefixes: all_prefixes,
    })
}

/// List all object keys under a prefix (handles pagination).
pub fn list_all_keys(prefix: &str) -> Result<Vec<String>, String> {
    let result = list_objects(prefix, None)?;
    Ok(result.objects.into_iter().map(|o| o.key).collect())
}

// ---------------------------------------------------------------------------
// HTTP helper
// ---------------------------------------------------------------------------

/// Make a signed S3 request.
///
/// `range` and `copy_source` are the two optional extra headers
/// needed across all operations. Using explicit params avoids
/// lifetime issues with waki's builder.
fn s3_request(
    method: &str,
    key: &str,
    query: &str,
    body: &[u8],
    range: Option<&str>,
    copy_source: Option<&str>,
) -> Result<waki::Response, String> {
    let c = &*CONFIG;
    let path = format!("/{}/{}", c.bucket, key);
    let url = if query.is_empty() {
        format!("{}{path}", c.endpoint)
    } else {
        format!("{}{path}?{query}", c.endpoint)
    };

    let host = c
        .endpoint
        .strip_prefix("https://")
        .or_else(|| c.endpoint.strip_prefix("http://"))
        .unwrap_or(&c.endpoint);

    // Headers for signing.
    let mut sign_headers = BTreeMap::new();
    sign_headers.insert("host".into(), host.to_string());
    if let Some(r) = range {
        sign_headers.insert("range".into(), r.to_string());
    }
    if let Some(cs) = copy_source {
        sign_headers.insert("x-amz-copy-source".into(), cs.to_string());
    }

    let payload_hash = sigv4::sha256_hex(body);
    let (amz_date, authorization, content_sha256) = sigv4::sign(
        method,
        &path,
        query,
        &sign_headers,
        &payload_hash,
        &c.access_key,
        &c.secret_key,
        &c.region,
    );

    let client = Client::new();
    let mut builder = match method {
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        _ => return Err(format!("unsupported method: {method}")),
    };
    builder = builder
        .header("Host", host)
        .header("x-amz-date", &amz_date)
        .header("x-amz-content-sha256", &content_sha256)
        .header("Authorization", &authorization);
    if let Some(r) = range {
        builder = builder.header("Range", r);
    }
    if let Some(cs) = copy_source {
        builder = builder.header("x-amz-copy-source", cs);
    }
    if !body.is_empty() {
        builder = builder.body(body);
    }
    builder.send().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// XML helpers (minimal, only for S3 responses)
// ---------------------------------------------------------------------------

fn xml_blocks<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut blocks = Vec::new();
    let mut search = xml;
    while let Some(start) = search.find(&open) {
        let content_start = start + open.len();
        if let Some(end) = search[content_start..].find(&close) {
            blocks.push(&search[content_start..content_start + end]);
            search = &search[content_start + end + close.len()..];
        } else {
            break;
        }
    }
    blocks
}

fn xml_tag_value<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(&xml[start..end])
}

fn error_message(status: u16, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    if let Some(msg) = xml_tag_value(&text, "Message") {
        format!("S3 error {status}: {msg}")
    } else {
        format!("S3 error {status}: {text}")
    }
}

// ---------------------------------------------------------------------------
// HTTP date parsing (RFC 7231: "Mon, 02 Jan 2006 15:04:05 GMT")
// ---------------------------------------------------------------------------

fn parse_http_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let day: u64 = parts[1].parse().ok()?;
    let month = match parts[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: u64 = parts[3].parse().ok()?;
    let time: Vec<&str> = parts[4].split(':').collect();
    if time.len() < 3 {
        return None;
    }
    let hour: u64 = time[0].parse().ok()?;
    let minute: u64 = time[1].parse().ok()?;
    let second: u64 = time[2].parse().ok()?;

    Some(civil_to_unix(year, month, day, hour, minute, second))
}

/// Convert civil datetime to Unix timestamp.
fn civil_to_unix(year: u64, month: u64, day: u64, hour: u64, min: u64, sec: u64) -> u64 {
    let (y, m) = if month <= 2 {
        (year as i64 - 1, month + 9)
    } else {
        (year as i64, month - 3)
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era as u64 * 146097 + doe - 719468;
    days * 86400 + hour * 3600 + min * 60 + sec
}
