use std::sync::LazyLock;

pub struct Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub endpoint: String,
    pub prefix: String,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());
    let endpoint = std::env::var("S3_ENDPOINT")
        .unwrap_or_else(|_| format!("https://s3.{region}.amazonaws.com"));
    let mut prefix = std::env::var("S3_PREFIX").unwrap_or_default();
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    Config {
        bucket: std::env::var("S3_BUCKET").expect("S3_BUCKET is required"),
        region,
        access_key: std::env::var("S3_ACCESS_KEY_ID").expect("S3_ACCESS_KEY_ID is required"),
        secret_key: std::env::var("S3_SECRET_ACCESS_KEY")
            .expect("S3_SECRET_ACCESS_KEY is required"),
        endpoint,
        prefix,
    }
});
