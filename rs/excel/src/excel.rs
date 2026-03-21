use calamine::{Data, Reader, Xlsx};
use sha2::{Digest, Sha256};
use std::io::Cursor;

pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

pub struct ExcelDocument {
    sheets: Vec<SheetData>,
}

pub struct SheetData {
    pub index: u32,
    pub name: String,
    pub rows: u32,
    pub cols: u32,
    csv: String,
}

impl ExcelDocument {
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let cursor = Cursor::new(data);
        let mut workbook: Xlsx<_> = Xlsx::new(cursor).map_err(|e| format!("xlsx: {e}"))?;
        let sheet_names = workbook.sheet_names().to_vec();
        let mut sheets = Vec::new();

        for (i, name) in sheet_names.iter().enumerate() {
            let range = workbook
                .worksheet_range(name)
                .map_err(|e| format!("sheet '{name}': {e}"))?;

            let (rows, cols) = range.get_size();
            let mut csv = String::new();
            for row in range.rows() {
                let line: Vec<String> =
                    row.iter().map(|c| csv_escape(&cell_to_string(c))).collect();
                csv.push_str(&line.join(","));
                csv.push('\n');
            }

            sheets.push(SheetData {
                index: i as u32,
                name: name.clone(),
                rows: rows as u32,
                cols: cols as u32,
                csv,
            });
        }

        Ok(ExcelDocument { sheets })
    }

    pub fn sheets(&self) -> &[SheetData] {
        &self.sheets
    }

    pub fn read_sheet_csv(&self, sheet_index: usize) -> Result<&str, String> {
        self.sheets
            .get(sheet_index)
            .map(|s| s.csv.as_str())
            .ok_or_else(|| {
                format!(
                    "sheet index {sheet_index} out of range (have {})",
                    self.sheets.len()
                )
            })
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        _ => cell.to_string(),
    }
}
