use crate::Component;
use crate::bindings::exports::asterai::twilio::api::Guest;
use crate::bindings::exports::asterai::twilio::types::User;
use base64::prelude::*;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};
use waki::Client;

const API_BASE: &str = "https://api.twilio.com/2010-04-01/Accounts";

static ACCOUNT_SID: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TWILIO_ACCOUNT_SID").expect("TWILIO_ACCOUNT_SID env var is required")
});
static AUTH_TOKEN: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN env var is required")
});
static PHONE_NUMBER: LazyLock<String> = LazyLock::new(|| {
    let num = std::env::var("TWILIO_PHONE_NUMBER").expect("TWILIO_PHONE_NUMBER env var is required");
    match num.starts_with('+') {
        true => num,
        false => format!("+{num}"),
    }
});
static SELF_INFO: OnceLock<PhoneNumberInfo> = OnceLock::new();

impl Guest for Component {
    fn get_self() -> User {
        let info = SELF_INFO.get_or_init(|| {
            fetch_self().unwrap_or_else(|_| PhoneNumberInfo {
                friendly_name: String::new(),
                phone_number: PHONE_NUMBER.clone(),
            })
        });
        User {
            name: info.friendly_name.clone(),
            phone: info.phone_number.clone(),
        }
    }

    fn send_message(content: String, to: String) -> String {
        send_sms(&content, &to).unwrap_or_else(|e| format!("error: {e}"))
    }
}

#[derive(Deserialize, Clone)]
struct PhoneNumberInfo {
    friendly_name: String,
    phone_number: String,
}

#[derive(Deserialize)]
struct IncomingPhoneNumbersResponse {
    incoming_phone_numbers: Vec<PhoneNumberEntry>,
}

#[derive(Deserialize)]
struct PhoneNumberEntry {
    sid: String,
    friendly_name: String,
    phone_number: String,
}

#[derive(Deserialize)]
struct SendResponse {
    sid: Option<String>,
    code: Option<u32>,
    message: Option<String>,
}

pub fn account_sid() -> &'static str {
    &ACCOUNT_SID
}

pub fn auth_token() -> &'static str {
    &AUTH_TOKEN
}

pub fn phone_number() -> &'static str {
    &PHONE_NUMBER
}

pub fn auth_header() -> String {
    let credentials = format!("{}:{}", account_sid(), auth_token());
    format!("Basic {}", BASE64_STANDARD.encode(credentials))
}

pub fn api_url(path: &str) -> String {
    format!("{}/{}/{}", API_BASE, account_sid(), path)
}

pub fn fetch_phone_number_sid() -> Result<String, String> {
    let entries = fetch_incoming_phone_numbers()?;
    entries
        .first()
        .map(|e| e.sid.clone())
        .ok_or_else(|| format!("phone number {} not found in account", phone_number()))
}

pub fn urlencode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => result.push_str(&format!("%{byte:02X}")),
        }
    }
    result
}

fn fetch_self() -> Result<PhoneNumberInfo, String> {
    let entries = fetch_incoming_phone_numbers()?;
    entries
        .into_iter()
        .next()
        .map(|e| PhoneNumberInfo {
            friendly_name: e.friendly_name,
            phone_number: e.phone_number,
        })
        .ok_or_else(|| format!("phone number {} not found", phone_number()))
}

fn fetch_incoming_phone_numbers() -> Result<Vec<PhoneNumberEntry>, String> {
    let url = api_url(&format!(
        "IncomingPhoneNumbers.json?PhoneNumber={}",
        urlencode(phone_number()),
    ));
    let response = Client::new()
        .get(&url)
        .header("Authorization", &auth_header())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let resp: IncomingPhoneNumbersResponse =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    Ok(resp.incoming_phone_numbers)
}

fn send_sms(content: &str, to: &str) -> Result<String, String> {
    let url = api_url("Messages.json");
    let body = format!(
        "From={}&To={}&Body={}",
        urlencode(phone_number()),
        urlencode(to),
        urlencode(content),
    );
    let response = Client::new()
        .post(&url)
        .header("Authorization", &auth_header())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.as_bytes())
        .send()
        .map_err(|e| format!("request failed: {e}"))?;
    let bytes = response
        .body()
        .map_err(|e| format!("read body failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let resp: SendResponse =
        serde_json::from_str(&text).map_err(|e| format!("parse response failed: {e}: {text}"))?;
    if let Some(code) = resp.code {
        let msg = resp.message.unwrap_or_else(|| "unknown error".into());
        return Err(format!("error {code}: {msg}"));
    }
    resp.sid
        .ok_or_else(|| "missing message SID in response".into())
}
