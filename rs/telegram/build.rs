use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

fn main() {
    // Generate a new webhook salt per build.
    let salt = RandomState::new().build_hasher().finish();
    println!("cargo:rustc-env=WEBHOOK_SALT={salt:x}");
}
