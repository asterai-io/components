use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;

use crate::bindings::exports::asterai::word::api::Guest;
use crate::bindings::exports::asterai::word::types::{Paragraph, TableRow};

mod docx;

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

static STORE: Mutex<Option<HashMap<String, docx::DocxDocument>>> = Mutex::new(None);

struct Component;

impl Guest for Component {
    fn load_binary(data: Vec<u8>) -> Result<String, String> {
        let hash = docx::sha256_hex(&data);
        with_store(|store| {
            if store.contains_key(&hash) {
                return Ok(hash);
            }
            let doc = docx::DocxDocument::from_bytes(&data)?;
            store.insert(hash.clone(), doc);
            Ok(hash)
        })
    }

    fn load_base64(data: String) -> Result<String, String> {
        let bytes = b64().decode(&data).map_err(|e| format!("base64: {e}"))?;
        Self::load_binary(bytes)
    }

    fn save_binary(doc_id: String) -> Result<Vec<u8>, String> {
        with_doc(&doc_id, |doc| doc.to_bytes())
    }

    fn save_base64(doc_id: String) -> Result<String, String> {
        let bytes = Self::save_binary(doc_id)?;
        Ok(b64().encode(&bytes))
    }

    fn unload(doc_id: String) -> Result<(), String> {
        with_store(|store| {
            store
                .remove(&doc_id)
                .ok_or_else(|| format!("document not found: {doc_id}"))?;
            Ok(())
        })
    }

    fn get_paragraphs(doc_id: String) -> Result<Vec<Paragraph>, String> {
        with_doc(&doc_id, |doc| {
            Ok(doc
                .get_paragraphs()
                .into_iter()
                .map(|(index, text)| Paragraph { index, text })
                .collect())
        })
    }

    fn set_paragraph_text(
        doc_id: String,
        paragraph_index: u32,
        text: String,
    ) -> Result<(), String> {
        with_doc(&doc_id, |doc| {
            doc.set_paragraph_text(paragraph_index as usize, &text)
        })
    }

    fn replace_text(doc_id: String, search: String, replace: String) -> Result<u32, String> {
        with_doc(&doc_id, |doc| Ok(doc.replace_text(&search, &replace)))
    }

    fn get_table_count(doc_id: String) -> Result<u32, String> {
        with_doc(&doc_id, |doc| Ok(doc.get_table_count()))
    }

    fn get_table_rows(doc_id: String, table_index: u32) -> Result<Vec<TableRow>, String> {
        with_doc(&doc_id, |doc| {
            Ok(doc
                .get_table_rows(table_index as usize)?
                .into_iter()
                .map(|cells| TableRow { cells })
                .collect())
        })
    }

    fn set_table_cell(
        doc_id: String,
        table_index: u32,
        row_index: u32,
        cell_index: u32,
        text: String,
    ) -> Result<(), String> {
        with_doc(&doc_id, |doc| {
            doc.set_table_cell(
                table_index as usize,
                row_index as usize,
                cell_index as usize,
                &text,
            )
        })
    }

    fn add_table_row(doc_id: String, table_index: u32, cells: Vec<String>) -> Result<(), String> {
        with_doc(&doc_id, |doc| {
            doc.add_table_row(table_index as usize, &cells)
        })
    }

    fn remove_table_row(doc_id: String, table_index: u32, row_index: u32) -> Result<(), String> {
        with_doc(&doc_id, |doc| {
            doc.remove_table_row(table_index as usize, row_index as usize)
        })
    }
}

bindings::export!(Component with_types_in bindings);

fn with_store<T>(f: impl FnOnce(&mut HashMap<String, docx::DocxDocument>) -> T) -> T {
    let mut guard = STORE.lock().unwrap();
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

fn with_doc<T>(
    doc_id: &str,
    f: impl FnOnce(&mut docx::DocxDocument) -> Result<T, String>,
) -> Result<T, String> {
    with_store(|store| {
        let doc = store
            .get_mut(doc_id)
            .ok_or_else(|| format!("document not found: {doc_id}"))?;
        f(doc)
    })
}
