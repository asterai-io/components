use crate::bindings::exports::asterai::fs::fs::Guest;
use crate::bindings::exports::asterai::fs::types::{Entry, EntryKind, Metadata};
use std::io::{Read as _, Seek, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

struct Component;

impl Guest for Component {
    fn read(path: String) -> Result<Vec<u8>, String> {
        let p = resolve(&path)?;
        std::fs::read(&p).map_err(|e| e.to_string())
    }

    fn read_range(path: String, offset: u64, length: u64) -> Result<Vec<u8>, String> {
        let p = resolve(&path)?;
        let mut file = std::fs::File::open(&p).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; length as usize];
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        buf.truncate(n);
        Ok(buf)
    }

    fn write(path: String, data: Vec<u8>) -> Result<(), String> {
        let p = resolve(&path)?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &data).map_err(|e| e.to_string())
    }

    fn append(path: String, data: Vec<u8>) -> Result<(), String> {
        let p = resolve(&path)?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
            .map_err(|e| e.to_string())?;
        file.write_all(&data).map_err(|e| e.to_string())
    }

    fn touch(path: String) -> Result<(), String> {
        let p = resolve(&path)?;
        if p.exists() {
            let file = std::fs::File::open(&p).map_err(|e| e.to_string())?;
            let times = std::fs::FileTimes::new().set_modified(SystemTime::now());
            file.set_times(times).map_err(|e| e.to_string())
        } else {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::File::create(&p).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    fn ls(path: String, recursive: bool) -> Result<Vec<Entry>, String> {
        let p = resolve(&path)?;
        if recursive {
            ls_recursive(&p, &p)
        } else {
            ls_flat(&p)
        }
    }

    fn mkdir(path: String) -> Result<(), String> {
        let p = resolve(&path)?;
        std::fs::create_dir_all(&p).map_err(|e| e.to_string())
    }

    fn rm(path: String, recursive: bool) -> Result<(), String> {
        let p = resolve(&path)?;
        let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
        if meta.is_dir() {
            if recursive {
                std::fs::remove_dir_all(&p).map_err(|e| e.to_string())
            } else {
                std::fs::remove_dir(&p).map_err(|e| e.to_string())
            }
        } else {
            std::fs::remove_file(&p).map_err(|e| e.to_string())
        }
    }

    fn cp(src: String, dst: String, recursive: bool) -> Result<(), String> {
        let s = resolve(&src)?;
        let d = resolve(&dst)?;
        let meta = std::fs::metadata(&s).map_err(|e| e.to_string())?;
        if meta.is_dir() {
            if !recursive {
                return Err("use recursive to copy directories".into());
            }
            cp_recursive(&s, &d)
        } else {
            if let Some(parent) = d.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::copy(&s, &d).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    fn mv(src: String, dst: String) -> Result<(), String> {
        let s = resolve(&src)?;
        let d = resolve(&dst)?;
        if let Some(parent) = d.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&s, &d).map_err(|e| e.to_string())
    }

    fn stat(path: String) -> Result<Metadata, String> {
        let p = resolve(&path)?;
        let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
        Ok(to_metadata(&meta))
    }

    fn exists(path: String) -> Result<bool, String> {
        let p = resolve(&path)?;
        Ok(p.exists())
    }
}

fn allowed_dirs() -> Vec<String> {
    std::env::var("ASTERAI_ALLOWED_DIRS")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn format_allowed(dirs: &[String]) -> String {
    if dirs.is_empty() {
        "none configured".to_string()
    } else {
        dirs.join(", ")
    }
}

fn resolve(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        match std::env::var("FS_ROOT") {
            Ok(root) => Path::new(&root).join(path),
            Err(_) => {
                let dirs = allowed_dirs();
                return Err(format!(
                    "relative paths not supported without FS_ROOT, \
                     use an absolute path from one of the allowed directories: {}",
                    format_allowed(&dirs),
                ));
            }
        }
    };
    let dirs = allowed_dirs();
    if !dirs.is_empty() && !dirs.iter().any(|d| absolute.starts_with(d)) {
        return Err(format!(
            "path {} is not within any allowed directory: {}",
            absolute.display(),
            format_allowed(&dirs),
        ));
    }
    Ok(absolute)
}

fn to_entry_kind(meta: &std::fs::Metadata) -> EntryKind {
    if meta.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}

fn to_metadata(meta: &std::fs::Metadata) -> Metadata {
    let last_modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    Metadata {
        size: meta.len(),
        kind: to_entry_kind(meta),
        last_modified,
    }
}

fn ls_flat(dir: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let meta = item.metadata().map_err(|e| e.to_string())?;
        entries.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            kind: to_entry_kind(&meta),
            size: meta.len(),
        });
    }
    Ok(entries)
}

fn ls_recursive(dir: &Path, base: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for item in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let meta = item.metadata().map_err(|e| e.to_string())?;
        let rel = item
            .path()
            .strip_prefix(base)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        entries.push(Entry {
            name: rel,
            kind: to_entry_kind(&meta),
            size: meta.len(),
        });
        if meta.is_dir() {
            entries.extend(ls_recursive(&item.path(), base)?);
        }
    }
    Ok(entries)
}

fn cp_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for item in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let dest_path = dst.join(item.file_name());
        let meta = item.metadata().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            cp_recursive(&item.path(), &dest_path)?;
        } else {
            std::fs::copy(&item.path(), &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

bindings::export!(Component with_types_in bindings);
