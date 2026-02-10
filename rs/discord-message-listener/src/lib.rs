use std::env;
use log::error;
use crate::bindings::asterai::host_ws;
use crate::bindings::asterai::host_ws::connection::Config;
use crate::bindings::exports::asterai::host_ws::incoming_message::{ConnectionId, Guest as IncomingMessageGuest};
use crate::bindings::exports::wasi::cli::run::Guest as RunGuest;

#[allow(warnings)]
mod bindings;

struct Component;

impl RunGuest for Component {
    fn run() -> Result<(), ()> {
        let listener_targets = env::var("DISCORD_LISTENER_TARGETS").map_err(|_| {
            eprintln!("missing DISCORD_LISTENER_TARGETS env var");
            ()
        })?.split(",").map(|s| s.to_owned()).collect::<Vec<String>>();
        // TODO ensure listener_targets implement the interface.
        let connection_config = Config {
            url: "".to_string(),
            headers: vec![],
            auto_reconnect: true,
        };
        host_ws::connection::connect(&connection_config).map_err(|e| {
            eprintln!("connection failed: {e:#?}");
            ()
        })?;
        Ok(())
    }
}

impl IncomingMessageGuest for Component {
    fn on_message(id: ConnectionId, data: Vec<u8>) {
        todo!()
    }

    fn on_close(id: ConnectionId, code: u16, reason: String) {
        // No action needed, host will try to auto-reconnect.
    }

    fn on_error(_id: ConnectionId, message: String) {
        eprintln!("error: {message}");
    }
}

bindings::export!(Component with_types_in bindings);
