use crate::bindings::exports::asterai::cli::command::Guest;

mod ls;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}


struct Component;

impl Guest for Component {
    fn run(args: String, stdin: Option<String>) -> Result<String, String> {
        let mut parts = args.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("");
        let cmd_args = parts.next().unwrap_or("");
        match cmd {
            "ls" => ls::run(cmd_args, stdin),
            _ => Err(format!("unknown command: {cmd}")),
        }
    }
}

bindings::export!(Component with_types_in bindings);
