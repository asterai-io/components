use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read, Write};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

pub struct DocxDocument {
    entries: Vec<(String, Vec<u8>)>,
    body: Body,
}

struct Body {
    prefix: String,
    elements: Vec<BodyElement>,
    suffix: String,
}

#[derive(Clone)]
enum BodyElement {
    Paragraph(Paragraph),
    Table(Table),
    Other(String),
}

#[derive(Clone)]
struct Paragraph {
    open_tag: String,
    properties: String,
    children: Vec<ParaChild>,
}

#[derive(Clone)]
enum ParaChild {
    Run { properties: String, text: String },
    Other(String),
}

#[derive(Clone)]
struct Table {
    open_tag: String,
    header: String,
    rows: Vec<TableRow>,
}

#[derive(Clone)]
struct TableRow {
    open_tag: String,
    properties: String,
    cells: Vec<TableCell>,
    trailing: String,
}

#[derive(Clone)]
struct TableCell {
    open_tag: String,
    properties: String,
    paragraphs: Vec<Paragraph>,
    other: String,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn opening_tag(e: &BytesStart) -> String {
    let qname = e.name();
    let name = String::from_utf8_lossy(qname.as_ref());
    let mut tag = format!("<{name}");
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let val = String::from_utf8_lossy(&attr.value);
        tag.push_str(&format!(" {key}=\"{val}\""));
    }
    tag.push('>');
    tag
}

fn empty_tag(e: &BytesStart) -> String {
    let qname = e.name();
    let name = String::from_utf8_lossy(qname.as_ref());
    let mut tag = format!("<{name}");
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let val = String::from_utf8_lossy(&attr.value);
        tag.push_str(&format!(" {key}=\"{val}\""));
    }
    tag.push_str("/>");
    tag
}

fn closing_tag(name: &[u8]) -> String {
    format!("</{}>", String::from_utf8_lossy(name))
}

fn capture_raw(
    reader: &mut Reader<&[u8]>,
    open_tag: String,
    buf: &mut Vec<u8>,
) -> Result<String, String> {
    let mut out = open_tag;
    let mut depth = 1u32;
    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match &event {
            Event::Start(e) => {
                depth += 1;
                out.push_str(&opening_tag(e));
            }
            Event::End(e) => {
                depth -= 1;
                out.push_str(&closing_tag(e.name().as_ref()));
                if depth == 0 {
                    break;
                }
            }
            Event::Empty(e) => out.push_str(&empty_tag(e)),
            Event::Text(e) => {
                out.push_str(&String::from_utf8_lossy(e.as_ref()));
            }
            Event::CData(e) => {
                out.push_str("<![CDATA[");
                out.push_str(&String::from_utf8_lossy(e.as_ref()));
                out.push_str("]]>");
            }
            Event::Comment(e) => {
                out.push_str("<!--");
                out.push_str(&String::from_utf8_lossy(e.as_ref()));
                out.push_str("-->");
            }
            Event::Eof => return Err("unexpected EOF in element".into()),
            _ => {}
        }
    }
    Ok(out)
}

// Paragraph helpers

impl Paragraph {
    fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|c| match c {
                ParaChild::Run { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn set_text(&mut self, new_text: &str) {
        let mut first = true;
        for child in &mut self.children {
            if let ParaChild::Run { text, .. } = child {
                if first {
                    *text = new_text.to_string();
                    first = false;
                } else {
                    text.clear();
                }
            }
        }
        if first {
            self.children.push(ParaChild::Run {
                properties: String::new(),
                text: new_text.to_string(),
            });
        }
    }

    fn replace_text(&mut self, search: &str, replace: &str) -> u32 {
        let current = self.text();
        if !current.contains(search) {
            return 0;
        }
        let count = current.matches(search).count() as u32;
        self.set_text(&current.replace(search, replace));
        count
    }

    fn serialize(&self) -> String {
        let mut out = self.open_tag.clone();
        out.push_str(&self.properties);
        for child in &self.children {
            match child {
                ParaChild::Run { properties, text } => {
                    out.push_str("<w:r>");
                    out.push_str(properties);
                    out.push_str("<w:t xml:space=\"preserve\">");
                    out.push_str(&xml_escape(text));
                    out.push_str("</w:t></w:r>");
                }
                ParaChild::Other(raw) => out.push_str(raw),
            }
        }
        out.push_str("</w:p>");
        out
    }
}

impl TableCell {
    fn text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|p| p.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn serialize(&self) -> String {
        let mut out = self.open_tag.clone();
        out.push_str(&self.properties);
        for p in &self.paragraphs {
            out.push_str(&p.serialize());
        }
        out.push_str(&self.other);
        out.push_str("</w:tc>");
        out
    }
}

impl TableRow {
    fn serialize(&self) -> String {
        let mut out = self.open_tag.clone();
        out.push_str(&self.properties);
        for cell in &self.cells {
            out.push_str(&cell.serialize());
        }
        out.push_str(&self.trailing);
        out.push_str("</w:tr>");
        out
    }
}

impl Table {
    fn serialize(&self) -> String {
        let mut out = self.open_tag.clone();
        out.push_str(&self.header);
        for row in &self.rows {
            out.push_str(&row.serialize());
        }
        out.push_str("</w:tbl>");
        out
    }
}

impl Body {
    fn serialize(&self) -> String {
        let mut out = self.prefix.clone();
        for elem in &self.elements {
            match elem {
                BodyElement::Paragraph(p) => out.push_str(&p.serialize()),
                BodyElement::Table(t) => out.push_str(&t.serialize()),
                BodyElement::Other(raw) => out.push_str(raw),
            }
        }
        out.push_str(&self.suffix);
        out
    }
}

// Parsing

fn parse_body(xml: &str) -> Result<Body, String> {
    let body_start = xml.find("<w:body").ok_or("missing <w:body>")?;
    let body_open_end = xml[body_start..].find('>').ok_or("malformed <w:body>")? + body_start + 1;
    let body_close = xml.rfind("</w:body>").ok_or("missing </w:body>")?;

    let prefix = xml[..body_open_end].to_string();
    let content = &xml[body_open_end..body_close];
    let suffix = xml[body_close..].to_string();

    let elements = parse_body_elements(content)?;
    Ok(Body {
        prefix,
        elements,
        suffix,
    })
}

fn parse_body_elements(xml: &str) -> Result<Vec<BodyElement>, String> {
    let mut reader = Reader::from_reader(xml.as_bytes());
    let mut buf = Vec::new();
    let mut elements = Vec::new();

    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:p" => elements.push(BodyElement::Paragraph(parse_paragraph(
                        &mut reader,
                        tag,
                        &mut buf,
                    )?)),
                    b"w:tbl" => {
                        elements.push(BodyElement::Table(parse_table(&mut reader, tag, &mut buf)?))
                    }
                    _ => {
                        elements.push(BodyElement::Other(capture_raw(&mut reader, tag, &mut buf)?))
                    }
                }
            }
            Event::Empty(e) => elements.push(BodyElement::Other(empty_tag(&e))),
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(elements)
}

fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    open_tag: String,
    buf: &mut Vec<u8>,
) -> Result<Paragraph, String> {
    let mut properties = String::new();
    let mut children = Vec::new();

    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::End(ref e) if e.name().as_ref() == b"w:p" => break,
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:pPr" => {
                        properties = capture_raw(reader, tag, buf)?;
                    }
                    b"w:r" => {
                        let (props, text) = parse_run(reader, buf)?;
                        children.push(ParaChild::Run {
                            properties: props,
                            text,
                        });
                    }
                    _ => {
                        let raw = capture_raw(reader, tag, buf)?;
                        children.push(ParaChild::Other(raw));
                    }
                }
            }
            Event::Empty(e) => children.push(ParaChild::Other(empty_tag(&e))),
            Event::Eof => return Err("unexpected EOF in paragraph".into()),
            _ => {}
        }
    }

    Ok(Paragraph {
        open_tag,
        properties,
        children,
    })
}

fn parse_run(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Result<(String, String), String> {
    let mut properties = String::new();
    let mut text = String::new();

    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::End(ref e) if e.name().as_ref() == b"w:r" => break,
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:rPr" => {
                        properties = capture_raw(reader, tag, buf)?;
                    }
                    b"w:t" => loop {
                        let inner = reader
                            .read_event_into(buf)
                            .map_err(|e| format!("xml: {e}"))?
                            .into_owned();
                        buf.clear();
                        match inner {
                            Event::Text(t) => {
                                text.push_str(
                                    &t.unescape().map_err(|e| format!("xml unescape: {e}"))?,
                                );
                            }
                            Event::End(ref e) if e.name().as_ref() == b"w:t" => break,
                            _ => {}
                        }
                    },
                    _ => {
                        let _ = capture_raw(reader, tag, buf)?;
                    }
                }
            }
            Event::Empty(_) => {}
            Event::Eof => return Err("unexpected EOF in run".into()),
            _ => {}
        }
    }

    Ok((properties, text))
}

fn parse_table(
    reader: &mut Reader<&[u8]>,
    open_tag: String,
    buf: &mut Vec<u8>,
) -> Result<Table, String> {
    let mut header = String::new();
    let mut rows = Vec::new();

    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::End(ref e) if e.name().as_ref() == b"w:tbl" => break,
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:tr" => rows.push(parse_table_row(reader, tag, buf)?),
                    _ => header.push_str(&capture_raw(reader, tag, buf)?),
                }
            }
            Event::Empty(e) => header.push_str(&empty_tag(&e)),
            Event::Eof => return Err("unexpected EOF in table".into()),
            _ => {}
        }
    }

    Ok(Table {
        open_tag,
        header,
        rows,
    })
}

fn parse_table_row(
    reader: &mut Reader<&[u8]>,
    open_tag: String,
    buf: &mut Vec<u8>,
) -> Result<TableRow, String> {
    let mut properties = String::new();
    let mut cells = Vec::new();
    let mut trailing = String::new();

    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::End(ref e) if e.name().as_ref() == b"w:tr" => break,
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:trPr" => properties = capture_raw(reader, tag, buf)?,
                    b"w:tc" => cells.push(parse_table_cell(reader, tag, buf)?),
                    _ => trailing.push_str(&capture_raw(reader, tag, buf)?),
                }
            }
            Event::Empty(e) => trailing.push_str(&empty_tag(&e)),
            Event::Eof => return Err("unexpected EOF in table row".into()),
            _ => {}
        }
    }

    Ok(TableRow {
        open_tag,
        properties,
        cells,
        trailing,
    })
}

fn parse_table_cell(
    reader: &mut Reader<&[u8]>,
    open_tag: String,
    buf: &mut Vec<u8>,
) -> Result<TableCell, String> {
    let mut properties = String::new();
    let mut paragraphs = Vec::new();
    let mut other = String::new();

    loop {
        let event = reader
            .read_event_into(buf)
            .map_err(|e| format!("xml: {e}"))?
            .into_owned();
        buf.clear();
        match event {
            Event::End(ref e) if e.name().as_ref() == b"w:tc" => break,
            Event::Start(e) => {
                let tag = opening_tag(&e);
                match e.name().as_ref() {
                    b"w:tcPr" => properties = capture_raw(reader, tag, buf)?,
                    b"w:p" => paragraphs.push(parse_paragraph(reader, tag, buf)?),
                    _ => other.push_str(&capture_raw(reader, tag, buf)?),
                }
            }
            Event::Empty(e) => other.push_str(&empty_tag(&e)),
            Event::Eof => return Err("unexpected EOF in table cell".into()),
            _ => {}
        }
    }

    Ok(TableCell {
        open_tag,
        properties,
        paragraphs,
        other,
    })
}

// DocxDocument public API

impl DocxDocument {
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| format!("invalid zip: {e}"))?;
        let mut entries = Vec::new();
        let mut doc_xml = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| format!("zip: {e}"))?;
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| format!("read: {e}"))?;
            if name == "word/document.xml" {
                doc_xml = Some(String::from_utf8(data).map_err(|e| format!("utf8: {e}"))?);
            } else {
                entries.push((name, data));
            }
        }

        let xml = doc_xml.ok_or("missing word/document.xml")?;
        let body = parse_body(&xml)?;
        Ok(DocxDocument { entries, body })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let buf = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, data) in &self.entries {
            zip.start_file(name.as_str(), opts)
                .map_err(|e| format!("zip: {e}"))?;
            zip.write_all(data).map_err(|e| format!("write: {e}"))?;
        }

        let xml = self.body.serialize();
        zip.start_file("word/document.xml", opts)
            .map_err(|e| format!("zip: {e}"))?;
        zip.write_all(xml.as_bytes())
            .map_err(|e| format!("write: {e}"))?;

        let result = zip.finish().map_err(|e| format!("zip: {e}"))?;
        Ok(result.into_inner())
    }

    pub fn get_paragraphs(&self) -> Vec<(u32, String)> {
        let mut result = Vec::new();
        let mut idx = 0u32;
        for elem in &self.body.elements {
            if let BodyElement::Paragraph(p) = elem {
                result.push((idx, p.text()));
                idx += 1;
            }
        }
        result
    }

    pub fn set_paragraph_text(&mut self, index: usize, text: &str) -> Result<(), String> {
        let mut idx = 0usize;
        for elem in &mut self.body.elements {
            if let BodyElement::Paragraph(p) = elem {
                if idx == index {
                    p.set_text(text);
                    return Ok(());
                }
                idx += 1;
            }
        }
        Err(format!("paragraph index {index} out of range (have {idx})"))
    }

    pub fn replace_text(&mut self, search: &str, replace: &str) -> u32 {
        let mut count = 0u32;
        for elem in &mut self.body.elements {
            match elem {
                BodyElement::Paragraph(p) => count += p.replace_text(search, replace),
                BodyElement::Table(t) => {
                    for row in &mut t.rows {
                        for cell in &mut row.cells {
                            for p in &mut cell.paragraphs {
                                count += p.replace_text(search, replace);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        count
    }

    pub fn get_table_count(&self) -> u32 {
        self.body
            .elements
            .iter()
            .filter(|e| matches!(e, BodyElement::Table(_)))
            .count() as u32
    }

    fn get_table(&self, index: usize) -> Result<&Table, String> {
        self.body
            .elements
            .iter()
            .filter_map(|e| {
                if let BodyElement::Table(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .nth(index)
            .ok_or_else(|| format!("table index {index} out of range"))
    }

    fn get_table_mut(&mut self, index: usize) -> Result<&mut Table, String> {
        self.body
            .elements
            .iter_mut()
            .filter_map(|e| {
                if let BodyElement::Table(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .nth(index)
            .ok_or_else(|| format!("table index {index} out of range"))
    }

    pub fn get_table_rows(&self, table_index: usize) -> Result<Vec<Vec<String>>, String> {
        let table = self.get_table(table_index)?;
        Ok(table
            .rows
            .iter()
            .map(|row| row.cells.iter().map(|c| c.text()).collect())
            .collect())
    }

    pub fn set_table_cell(
        &mut self,
        table_idx: usize,
        row_idx: usize,
        cell_idx: usize,
        text: &str,
    ) -> Result<(), String> {
        let table = self.get_table_mut(table_idx)?;
        let row = table
            .rows
            .get_mut(row_idx)
            .ok_or_else(|| format!("row index {row_idx} out of range"))?;
        let cell = row
            .cells
            .get_mut(cell_idx)
            .ok_or_else(|| format!("cell index {cell_idx} out of range"))?;
        if let Some(p) = cell.paragraphs.first_mut() {
            p.set_text(text);
        } else {
            cell.paragraphs.push(Paragraph {
                open_tag: "<w:p>".into(),
                properties: String::new(),
                children: vec![ParaChild::Run {
                    properties: String::new(),
                    text: text.to_string(),
                }],
            });
        }
        Ok(())
    }

    pub fn add_table_row(&mut self, table_idx: usize, cells: &[String]) -> Result<(), String> {
        let table = self.get_table_mut(table_idx)?;
        let template = table
            .rows
            .last()
            .ok_or("table has no rows to use as template")?
            .clone();
        let mut new_row = template;
        for (i, cell) in new_row.cells.iter_mut().enumerate() {
            let text = cells.get(i).map(|s| s.as_str()).unwrap_or("");
            if let Some(p) = cell.paragraphs.first_mut() {
                p.set_text(text);
            }
        }
        table.rows.push(new_row);
        Ok(())
    }

    pub fn remove_table_row(&mut self, table_idx: usize, row_idx: usize) -> Result<(), String> {
        let table = self.get_table_mut(table_idx)?;
        if row_idx >= table.rows.len() {
            return Err(format!(
                "row index {row_idx} out of range (have {})",
                table.rows.len()
            ));
        }
        table.rows.remove(row_idx);
        Ok(())
    }
}
