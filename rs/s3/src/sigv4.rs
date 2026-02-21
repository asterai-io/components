use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::SystemTime;

type HmacSha256 = Hmac<Sha256>;

pub fn sha256_hex(data: &[u8]) -> String {
    hex_encode(&Sha256::digest(data))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// URI-encode a string per AWS requirements.
pub fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b'/' if !encode_slash => out.push('/'),
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// Format a Unix timestamp as YYYYMMDDTHHMMSSZ.
fn format_timestamp(ts: u64) -> (String, String) {
    let secs = ts;
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    // Civil date from day count (Howard Hinnant's algorithm).
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let date = format!("{y:04}{m:02}{d:02}");
    let datetime = format!("{date}T{hour:02}{minute:02}{second:02}Z");
    (date, datetime)
}

/// Returns (amz_date, authorization_header, content_sha256).
pub fn sign(
    method: &str,
    path: &str,
    query: &str,
    headers: &BTreeMap<String, String>,
    payload_hash: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
) -> (String, String, String) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (date, datetime) = format_timestamp(now);
    let mut signed_headers: BTreeMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v.trim().to_string()))
        .collect();
    signed_headers.insert("x-amz-date".into(), datetime.clone());
    signed_headers.insert("x-amz-content-sha256".into(), payload_hash.into());
    // Canonical headers.
    let canonical_headers: String = signed_headers
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect();
    let signed_header_names: String = signed_headers
        .keys()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(";");
    // Canonical query string — values are already URI-encoded by the caller,
    // so we only need to sort by parameter name.
    let canonical_query = if query.is_empty() {
        String::new()
    } else {
        let mut pairs: Vec<&str> = query.split('&').collect();
        pairs.sort();
        pairs.join("&")
    };
    let canonical_request = format!(
        "{method}\n{}\n{canonical_query}\n{canonical_headers}\n{signed_header_names}\n{payload_hash}",
        uri_encode(path, false),
    );
    let scope = format!("{date}/{region}/s3/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{datetime}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes()),
    );
    // Derive signing key.
    let k_date = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex_encode(&hmac_sha256(&k_signing, string_to_sign.as_bytes()));
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_header_names}, Signature={signature}"
    );
    (datetime, authorization, payload_hash.into())
}
