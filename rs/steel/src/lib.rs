use crate::bindings::exports::asterai::steel::steel::Guest;
use crate::bindings::exports::asterai::steel::types::{
    PdfOptions, ScrapeOptions, ScreenshotOptions, SessionOptions,
};

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

mod api;

struct Component;

impl Guest for Component {
    fn create_session(options: SessionOptions) -> String {
        api::create_session(&options).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn release_session(session_id: String) -> String {
        api::release_session(&session_id).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn release_all_sessions() -> String {
        api::release_all_sessions().unwrap_or_else(|e| format!("error: {e}"))
    }

    fn get_session(session_id: String) -> String {
        api::get_session(&session_id).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn list_sessions() -> String {
        api::list_sessions().unwrap_or_else(|e| format!("error: {e}"))
    }

    fn scrape(options: ScrapeOptions) -> String {
        api::scrape(&options).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn screenshot(options: ScreenshotOptions) -> String {
        api::screenshot(&options).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn pdf(options: PdfOptions) -> String {
        api::pdf(&options).unwrap_or_else(|e| format!("error: {e}"))
    }
}

bindings::export!(Component with_types_in bindings);
