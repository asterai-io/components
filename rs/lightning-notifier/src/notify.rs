use crate::bindings::asterai::host::api;
use std::sync::LazyLock;

static NOTIFY_COMPONENT: LazyLock<String> =
    LazyLock::new(|| std::env::var("NOTIFY_COMPONENT").expect("NOTIFY_COMPONENT is required"));

static NOTIFY_FUNCTION: LazyLock<String> = LazyLock::new(|| {
    std::env::var("NOTIFY_FUNCTION").unwrap_or_else(|_| "api/send-message".to_string())
});

static NOTIFY_RECIPIENTS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("NOTIFY_RECIPIENTS")
        .expect("NOTIFY_RECIPIENTS is required")
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
});

pub fn validate_env() -> Result<(), String> {
    if std::env::var("NOTIFY_COMPONENT")
        .unwrap_or_default()
        .is_empty()
    {
        return Err("NOTIFY_COMPONENT env var is required".to_string());
    }
    let recipients: Vec<_> = std::env::var("NOTIFY_RECIPIENTS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if recipients.is_empty() {
        return Err("NOTIFY_RECIPIENTS env var is required".to_string());
    }
    Ok(())
}

pub fn send(message: &str) {
    let message = crate::llm::rewrite(message);
    for recipient in NOTIFY_RECIPIENTS.iter() {
        let args = serde_json::json!([&message, recipient]);
        let args_str = args.to_string();
        match api::call_component_function(&NOTIFY_COMPONENT, &NOTIFY_FUNCTION, &args_str) {
            Ok(_) => println!("lightning-notifier: sent to {recipient}: {message}"),
            Err(e) => eprintln!(
                "lightning-notifier: notify to {recipient} failed ({:?}): {}",
                e.kind, e.message
            ),
        }
    }
}
