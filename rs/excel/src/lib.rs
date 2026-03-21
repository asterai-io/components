use std::collections::HashMap;
use std::sync::Mutex;

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

static STORE: Mutex<Option<HashMap<String, excel::ExcelDocument>>> = Mutex::new(None);

fn with_store<T>(f: impl FnOnce(&mut HashMap<String, excel::ExcelDocument>) -> T) -> T {
    let mut guard = STORE.lock().unwrap();
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

fn with_doc<T>(
    doc_id: &str,
    f: impl FnOnce(&excel::ExcelDocument) -> Result<T, String>,
) -> Result<T, String> {
    with_store(|store| {
        let doc = store
            .get(doc_id)
            .ok_or_else(|| format!("document not found: {doc_id}"))?;
        f(doc)
    })
}

struct Component;

impl Guest for Component {
    fn load_binary(data: Vec<u8>) -> Result<String, String> {
        let hash = excel::sha256_hex(&data);
        with_store(|store| {
            if store.contains_key(&hash) {
                return Ok(hash);
            }
            let doc = excel::ExcelDocument::from_bytes(&data)?;
            store.insert(hash.clone(), doc);
            Ok(hash)
        })
    }

    fn load_base64(data: String) -> Result<String, String> {
        let bytes = b64().decode(&data).map_err(|e| format!("base64: {e}"))?;
        Self::load_binary(bytes)
    }

    fn unload(doc_id: String) -> Result<(), String> {
        with_store(|store| {
            store
                .remove(&doc_id)
                .ok_or_else(|| format!("document not found: {doc_id}"))?;
            Ok(())
        })
    }

    fn get_sheets(doc_id: String) -> Result<Vec<SheetInfo>, String> {
        with_doc(&doc_id, |doc| {
            Ok(doc
                .sheets()
                .iter()
                .map(|s| SheetInfo {
                    index: s.index,
                    name: s.name.clone(),
                    rows: s.rows,
                    cols: s.cols,
                })
                .collect())
        })
    }

    fn read_sheet_csv(doc_id: String, sheet_index: u32) -> Result<String, String> {
        with_doc(&doc_id, |doc| {
            doc.read_sheet_csv(sheet_index as usize)
                .map(|s| s.to_string())
        })
    }
}

bindings::export!(Component with_types_in bindings);
