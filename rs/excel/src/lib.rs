use base64::Engine;

use crate::bindings::exports::asterai::excel::api::Guest;
use crate::bindings::exports::asterai::excel::types::SheetInfo;

mod excel;

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

struct Component;

fn parse(data: &[u8]) -> Result<Vec<SheetInfo>, String> {
    let doc = excel::ExcelDocument::from_bytes(data)?;
    Ok(doc
        .sheets()
        .iter()
        .map(|s| SheetInfo {
            index: s.index,
            name: s.name.clone(),
            rows: s.rows,
            cols: s.cols,
            csv: s.csv().to_string(),
        })
        .collect())
}

impl Guest for Component {
    fn parse_binary(data: Vec<u8>) -> Result<Vec<SheetInfo>, String> {
        parse(&data)
    }

    fn parse_base64(data: String) -> Result<Vec<SheetInfo>, String> {
        let bytes = b64()
            .decode(&data)
            .map_err(|e| format!("base64: {e}"))?;
        parse(&bytes)
    }
}

bindings::export!(Component with_types_in bindings);
