use crate::bindings::exports::asterai::cli::common::Guest;

mod cat;
mod cp;
mod find;
mod grep;
mod head;
mod jq;
mod ls;
mod mkdir;
mod mv;
mod rm;
mod sed;
mod tail;
mod touch;
mod tree;

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
    fn ls(args: String, stdin: Option<String>) -> Result<String, String> {
        ls::run(&args, stdin)
    }
    fn cat(args: String, stdin: Option<String>) -> Result<String, String> {
        cat::run(&args, stdin)
    }
    fn cp(args: String, stdin: Option<String>) -> Result<String, String> {
        cp::run(&args, stdin)
    }
    fn mv(args: String, stdin: Option<String>) -> Result<String, String> {
        mv::run(&args, stdin)
    }
    fn rm(args: String, stdin: Option<String>) -> Result<String, String> {
        rm::run(&args, stdin)
    }
    fn mkdir(args: String, stdin: Option<String>) -> Result<String, String> {
        mkdir::run(&args, stdin)
    }
    fn touch(args: String, stdin: Option<String>) -> Result<String, String> {
        touch::run(&args, stdin)
    }
    fn grep(args: String, stdin: Option<String>) -> Result<String, String> {
        grep::run(&args, stdin)
    }
    fn sed(args: String, stdin: Option<String>) -> Result<String, String> {
        sed::run(&args, stdin)
    }
    fn jq(args: String, stdin: Option<String>) -> Result<String, String> {
        jq::run(&args, stdin)
    }
    fn find(args: String, stdin: Option<String>) -> Result<String, String> {
        find::run(&args, stdin)
    }
    fn head(args: String, stdin: Option<String>) -> Result<String, String> {
        head::run(&args, stdin)
    }
    fn tail(args: String, stdin: Option<String>) -> Result<String, String> {
        tail::run(&args, stdin)
    }
    fn tree(args: String, stdin: Option<String>) -> Result<String, String> {
        tree::run(&args, stdin)
    }
}

bindings::export!(Component with_types_in bindings);
