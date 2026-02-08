use crate::bindings::exports::asterai::firecrawl::firecrawl::Guest;

#[allow(warnings)]
mod bindings;

mod scrape;
mod search;

struct Component;

impl Guest for Component {
    fn scrape(url: String) -> String {
        scrape::scrape(&url).unwrap_or_else(|e| format!("error: {e}"))
    }

    fn search(query: String, limit: u32) -> String {
        search::search(&query, limit).unwrap_or_else(|e| format!("error: {e}"))
    }
}

bindings::export!(Component with_types_in bindings);
