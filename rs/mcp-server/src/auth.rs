use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use std::env;
use std::sync::LazyLock;

const SECRET_ENV: &str = "MCP_SERVER_SECRET";

static AUTH_CONFIG: LazyLock<Option<AuthConfig>> = LazyLock::new(|| {
    let secret = env::var(SECRET_ENV).ok()?;
    if secret.is_empty() {
        return None;
    }
    let token = sha256_hex(&secret);
    Some(AuthConfig { secret, token })
});

struct AuthConfig {
    secret: String,
    token: String,
}

pub fn is_auth_enabled() -> bool {
    AUTH_CONFIG.is_some()
}

pub fn verify_bearer(header_value: &str) -> bool {
    let Some(config) = AUTH_CONFIG.as_ref() else {
        return true;
    };
    let token = header_value.strip_prefix("Bearer ").unwrap_or(header_value);
    token == config.token
}

pub fn access_token() -> Option<&'static str> {
    AUTH_CONFIG.as_ref().map(|c| c.token.as_str())
}

/// Generates a stateless authorization code.
/// Format: `{code_challenge}.{hex(SHA256(code_challenge + ":" + secret))}`.
pub fn generate_auth_code(code_challenge: &str) -> Option<String> {
    let config = AUTH_CONFIG.as_ref()?;
    let sig = sha256_hex(&format!("{code_challenge}:{}", config.secret));
    Some(format!("{code_challenge}.{sig}"))
}

/// Verifies an authorization code and PKCE code_verifier.
/// Also validates client_secret if provided.
pub fn verify_auth_code(code: &str, code_verifier: &str, client_secret: Option<&str>) -> bool {
    let Some(config) = AUTH_CONFIG.as_ref() else {
        return false;
    };
    if let Some(cs) = client_secret {
        if cs != config.secret {
            return false;
        }
    }
    let Some((challenge, sig)) = code.split_once('.') else {
        return false;
    };
    let expected_sig = sha256_hex(&format!("{challenge}:{}", config.secret));
    if sig != expected_sig {
        return false;
    }
    let verifier_hash = Sha256::digest(code_verifier.as_bytes());
    let expected_challenge = URL_SAFE_NO_PAD.encode(verifier_hash);
    challenge == expected_challenge
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex("test-secret");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_auth_code_roundtrip() {
        // Can't test with AUTH_CONFIG (needs env var), but test the hashing logic
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(b"test-verifier"));
        assert!(!challenge.is_empty());
        assert!(!challenge.contains('='));
    }
}
