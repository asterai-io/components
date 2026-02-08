use crate::bindings::exports::asterai::fs_test::fs_test::Guest;
use std::fs;
use std::path::Path;

#[allow(warnings)]
mod bindings;

struct Component;

impl Guest for Component {
    fn increment_counter(dir: String) -> u64 {
        let path = Path::new(&dir).join("counter.txt");
        let current: u64 = fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let next = current + 1;
        fs::write(&path, next.to_string()).expect("failed to write counter.txt");
        println!("counter: {current} -> {next}");
        next
    }
}

bindings::export!(Component with_types_in bindings);
