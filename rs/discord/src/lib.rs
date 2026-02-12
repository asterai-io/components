use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;
use crate::listener::initialise_ws_client;

pub struct Component;

mod api;
mod listener;

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
        initialise_ws_client()?;
        Ok(())
    }
}

bindings::export!(Component with_types_in bindings);
