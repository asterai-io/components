const MAX_BACKOFF_SECS: u64 = 120;

pub enum RequestOutcome {
    Success(String),
    /// Eligible for retry (e.g. rate-limited or forbidden).
    Retryable(u16, String),
    Failure(String),
}

/// Retries `f` with exponential backoff on 429/403, giving up after 2 minutes.
/// On failure, returns the last error body from the server.
pub fn retry_with_exp_backoff<F>(mut f: F) -> Result<String, String>
where
    F: FnMut() -> Result<RequestOutcome, String>,
{
    let mut backoff_secs = 1;
    let mut total_waited = 0;
    loop {
        match f()? {
            RequestOutcome::Success(text) => return Ok(text),
            RequestOutcome::Retryable(status, _) if total_waited < MAX_BACKOFF_SECS => {
                eprintln!(
                    "retryable error ({}), retrying in {}s...",
                    status, backoff_secs
                );
                std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
                total_waited += backoff_secs;
                backoff_secs = (backoff_secs * 2)
                    .min(MAX_BACKOFF_SECS - total_waited)
                    .max(1);
            }
            RequestOutcome::Retryable(_, body) | RequestOutcome::Failure(body) => {
                return Err(body);
            }
        }
    }
}
