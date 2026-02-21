use crate::bindings::exports::asterai::fs::fs::Guest;
use crate::bindings::exports::asterai::fs::types::{Entry, EntryKind, Metadata};
use std::path::Path;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

pub mod config;
mod s3;
mod sigv4;

struct Component;

/// Validate path and return the full S3 key (prefix + path).
fn resolve_key(path: &str) -> Result<String, String> {
    for component in Path::new(path).components() {
        match component {
            std::path::Component::ParentDir => return Err(".. not allowed in path".into()),
            std::path::Component::RootDir => return Err("absolute paths not allowed".into()),
            std::path::Component::Prefix(_) => return Err("path prefixes not allowed".into()),
            _ => {}
        }
    }
    let prefix = &config::CONFIG.prefix;
    Ok(format!("{prefix}{path}"))
}

impl Guest for Component {
    fn read(path: String) -> Result<Vec<u8>, String> {
        let key = resolve_key(&path)?;
        s3::get_object(&key)
    }

    fn read_range(path: String, offset: u64, length: u64) -> Result<Vec<u8>, String> {
        let key = resolve_key(&path)?;
        s3::get_object_range(&key, offset, length)
    }

    fn write(path: String, data: Vec<u8>) -> Result<(), String> {
        let key = resolve_key(&path)?;
        s3::put_object(&key, &data)
    }

    fn append(path: String, data: Vec<u8>) -> Result<(), String> {
        let key = resolve_key(&path)?;
        let mut existing = match s3::get_object(&key) {
            Ok(d) => d,
            Err(_) => Vec::new(),
        };
        existing.extend_from_slice(&data);
        s3::put_object(&key, &existing)
    }

    fn touch(path: String) -> Result<(), String> {
        let key = resolve_key(&path)?;
        if s3::head_object(&key).is_ok() {
            s3::copy_object(&key, &key)
        } else {
            s3::put_object(&key, &[])
        }
    }

    fn ls(path: String, recursive: bool) -> Result<Vec<Entry>, String> {
        let key = resolve_key(&path)?;
        let prefix = if key.is_empty() || key.ends_with('/') {
            key
        } else {
            format!("{key}/")
        };
        let delimiter = if recursive { None } else { Some("/") };
        let result = s3::list_objects(&prefix, delimiter)?;
        let mut entries = Vec::new();
        for obj in result.objects {
            let name = obj.key.strip_prefix(&prefix).unwrap_or(&obj.key);
            if name.is_empty() {
                continue;
            }
            entries.push(Entry {
                name: name.to_string(),
                kind: EntryKind::File,
                size: obj.size,
            });
        }
        for p in result.common_prefixes {
            let name = p.strip_prefix(&prefix).unwrap_or(&p);
            let name = name.strip_suffix('/').unwrap_or(name);
            if name.is_empty() {
                continue;
            }
            entries.push(Entry {
                name: name.to_string(),
                kind: EntryKind::Directory,
                size: 0,
            });
        }
        Ok(entries)
    }

    fn mkdir(_path: String) -> Result<(), String> {
        Ok(())
    }

    fn rm(path: String, recursive: bool) -> Result<(), String> {
        let key = resolve_key(&path)?;
        if recursive {
            let prefix = if key.ends_with('/') {
                key.clone()
            } else {
                format!("{key}/")
            };
            let objects = s3::list_all_keys(&prefix)?;
            for k in &objects {
                s3::delete_object(k)?;
            }
            // Also delete the key itself (could be a 0-byte dir marker).
            let _ = s3::delete_object(&key);
            Ok(())
        } else {
            s3::delete_object(&key)
        }
    }

    fn cp(src: String, dst: String, recursive: bool) -> Result<(), String> {
        let src_key = resolve_key(&src)?;
        let dst_key = resolve_key(&dst)?;
        if !recursive {
            return s3::copy_object(&src_key, &dst_key);
        }
        let src_prefix = if src_key.ends_with('/') {
            src_key.clone()
        } else {
            format!("{src_key}/")
        };
        let dst_prefix = if dst_key.ends_with('/') {
            dst_key.clone()
        } else {
            format!("{dst_key}/")
        };
        let keys = s3::list_all_keys(&src_prefix)?;
        for k in &keys {
            let rel = k.strip_prefix(&src_prefix).unwrap_or(k);
            let new_key = format!("{dst_prefix}{rel}");
            s3::copy_object(k, &new_key)?;
        }
        Ok(())
    }

    fn mv(src: String, dst: String) -> Result<(), String> {
        let src_key = resolve_key(&src)?;
        let dst_key = resolve_key(&dst)?;
        if s3::head_object(&src_key).is_ok() {
            s3::copy_object(&src_key, &dst_key)?;
            s3::delete_object(&src_key)
        } else {
            let src_prefix = format!("{src_key}/");
            let dst_prefix = format!("{dst_key}/");
            let keys = s3::list_all_keys(&src_prefix)?;
            if keys.is_empty() {
                return Err("source not found".into());
            }
            for k in &keys {
                let rel = k.strip_prefix(&src_prefix).unwrap_or(k);
                let new_key = format!("{dst_prefix}{rel}");
                s3::copy_object(k, &new_key)?;
                s3::delete_object(k)?;
            }
            Ok(())
        }
    }

    fn stat(path: String) -> Result<Metadata, String> {
        let key = resolve_key(&path)?;
        let head = s3::head_object(&key)?;
        Ok(Metadata {
            size: head.content_length,
            kind: EntryKind::File,
            last_modified: head.last_modified,
        })
    }

    fn exists(path: String) -> Result<bool, String> {
        let key = resolve_key(&path)?;
        Ok(s3::head_object(&key).is_ok())
    }
}

bindings::export!(Component with_types_in bindings);
