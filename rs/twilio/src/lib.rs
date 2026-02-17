use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;

pub struct Component;

mod api;
mod webhook;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

impl RunGuest for Component {
    fn run() -> Result<(), ()> {
        let handlers = webhook::parse_handlers();
        if handlers.is_empty() {
            println!("twilio: no handlers configured, send-only mode");
            return Ok(());
        }
        webhook::validate_handlers(&handlers)?;
        webhook::setup_webhook()?;
        println!("twilio: ready");
        Ok(())
    }
}

bindings::export!(Component with_types_in bindings);
