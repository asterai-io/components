use crate::bindings::asterai::host::api;
use crate::bindings::exports::asterai::llm::llm::Guest;

#[allow(warnings)]
mod bindings;

struct Component;

impl Guest for Component {
    fn greet(name: String) {
        let greeting = format!("hello {name}!");
        println!("{greeting}");
    }
}

bindings::export!(Component with_types_in bindings);
