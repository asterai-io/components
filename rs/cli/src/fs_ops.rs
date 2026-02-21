#[allow(dead_code)]
pub enum EntryKind {
    File,
    Directory,
}

pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
}

pub struct Metadata {
    pub size: u64,
    pub kind: EntryKind,
    pub last_modified: Option<u64>,
}

impl Metadata {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use crate::bindings::asterai::fs::fs as wit_fs;

    fn convert_entry(e: crate::bindings::asterai::fs::types::Entry) -> Entry {
        use crate::bindings::asterai::fs::types::EntryKind as WitKind;
        Entry {
            name: e.name,
            kind: match e.kind {
                WitKind::File => EntryKind::File,
                WitKind::Directory => EntryKind::Directory,
            },
            size: e.size,
        }
    }

    fn convert_metadata(m: crate::bindings::asterai::fs::types::Metadata) -> Metadata {
        use crate::bindings::asterai::fs::types::EntryKind as WitKind;
        Metadata {
            size: m.size,
            kind: match m.kind {
                WitKind::File => EntryKind::File,
                WitKind::Directory => EntryKind::Directory,
            },
            last_modified: m.last_modified,
        }
    }

    pub fn read(path: &str) -> Result<Vec<u8>, String> {
        wit_fs::read(path)
    }

    pub fn write(path: &str, data: &[u8]) -> Result<(), String> {
        wit_fs::write(path, data)
    }

    pub fn append(path: &str, data: &[u8]) -> Result<(), String> {
        wit_fs::append(path, data)
    }

    pub fn touch(path: &str) -> Result<(), String> {
        wit_fs::touch(path)
    }

    pub fn ls(path: &str, recursive: bool) -> Result<Vec<Entry>, String> {
        wit_fs::ls(path, recursive).map(|v| v.into_iter().map(convert_entry).collect())
    }

    pub fn mkdir(path: &str) -> Result<(), String> {
        wit_fs::mkdir(path)
    }

    pub fn rm(path: &str, recursive: bool) -> Result<(), String> {
        wit_fs::rm(path, recursive)
    }

    pub fn cp(src: &str, dst: &str, recursive: bool) -> Result<(), String> {
        wit_fs::cp(src, dst, recursive)
    }

    pub fn mv(src: &str, dst: &str) -> Result<(), String> {
        wit_fs::mv(src, dst)
    }

    pub fn stat(path: &str) -> Result<Metadata, String> {
        wit_fs::stat(path).map(convert_metadata)
    }

    pub fn exists(path: &str) -> Result<bool, String> {
        wit_fs::exists(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::fs;
    use std::time::SystemTime;

    pub fn read(path: &str) -> Result<Vec<u8>, String> {
        fs::read(path).map_err(|e| e.to_string())
    }

    pub fn write(path: &str, data: &[u8]) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn append(path: &str, data: &[u8]) -> Result<(), String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        f.write_all(data).map_err(|e| e.to_string())
    }

    pub fn touch(path: &str) -> Result<(), String> {
        if fs::metadata(path).is_ok() {
            let file = fs::File::open(path).map_err(|e| e.to_string())?;
            let times = fs::FileTimes::new().set_modified(SystemTime::now());
            file.set_times(times).map_err(|e| e.to_string())
        } else {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            fs::File::create(path).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    pub fn ls(path: &str, recursive: bool) -> Result<Vec<Entry>, String> {
        let mut entries = Vec::new();
        ls_inner(path, "", recursive, &mut entries)?;
        Ok(entries)
    }

    fn ls_inner(
        base: &str,
        prefix: &str,
        recursive: bool,
        entries: &mut Vec<Entry>,
    ) -> Result<(), String> {
        let dir = if prefix.is_empty() {
            base.to_string()
        } else {
            format!("{base}/{prefix}")
        };
        let rd = fs::read_dir(&dir).map_err(|e| format!("{dir}: {e}"))?;
        let mut items: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        items.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        for item in items {
            let name = item.file_name().to_string_lossy().into_owned();
            let meta = item.metadata().map_err(|e| e.to_string())?;
            let relative = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            entries.push(Entry {
                name: if recursive { relative.clone() } else { name.clone() },
                kind: if meta.is_dir() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                },
                size: meta.len(),
            });
            if recursive && meta.is_dir() {
                ls_inner(base, &relative, true, entries)?;
            }
        }
        Ok(())
    }

    pub fn mkdir(path: &str) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    pub fn rm(path: &str, recursive: bool) -> Result<(), String> {
        let meta = fs::metadata(path).map_err(|e| e.to_string())?;
        if meta.is_dir() {
            if recursive {
                fs::remove_dir_all(path).map_err(|e| e.to_string())
            } else {
                fs::remove_dir(path).map_err(|e| e.to_string())
            }
        } else {
            fs::remove_file(path).map_err(|e| e.to_string())
        }
    }

    pub fn cp(src: &str, dst: &str, recursive: bool) -> Result<(), String> {
        let meta = fs::metadata(src).map_err(|e| e.to_string())?;
        if meta.is_dir() {
            if !recursive {
                return Err(format!("cp: {src}: is a directory"));
            }
            cp_recursive(std::path::Path::new(src), std::path::Path::new(dst))
        } else {
            if let Some(parent) = std::path::Path::new(dst).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            fs::copy(src, dst).map_err(|e| e.to_string())?;
            Ok(())
        }
    }

    fn cp_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let target = dst.join(entry.file_name());
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.is_dir() {
                cp_recursive(&entry.path(), &target)?;
            } else {
                fs::copy(&entry.path(), &target).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn mv(src: &str, dst: &str) -> Result<(), String> {
        fs::rename(src, dst).map_err(|e| e.to_string())
    }

    pub fn stat(path: &str) -> Result<Metadata, String> {
        let meta = fs::metadata(path).map_err(|e| e.to_string())?;
        let last_modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        Ok(Metadata {
            size: meta.len(),
            kind: if meta.is_dir() {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            last_modified,
        })
    }

    pub fn exists(path: &str) -> Result<bool, String> {
        Ok(std::path::Path::new(path).exists())
    }
}

pub fn read(path: &str) -> Result<Vec<u8>, String> {
    imp::read(path)
}

pub fn read_to_string(path: &str) -> Result<String, String> {
    let bytes = imp::read(path)?;
    String::from_utf8(bytes).map_err(|e| format!("{path}: invalid UTF-8: {e}"))
}

pub fn write(path: &str, data: &[u8]) -> Result<(), String> {
    imp::write(path, data)
}

pub fn append(path: &str, data: &[u8]) -> Result<(), String> {
    imp::append(path, data)
}

pub fn touch(path: &str) -> Result<(), String> {
    imp::touch(path)
}

pub fn ls(path: &str, recursive: bool) -> Result<Vec<Entry>, String> {
    imp::ls(path, recursive)
}

pub fn mkdir(path: &str) -> Result<(), String> {
    imp::mkdir(path)
}

pub fn rm(path: &str, recursive: bool) -> Result<(), String> {
    imp::rm(path, recursive)
}

pub fn cp(src: &str, dst: &str, recursive: bool) -> Result<(), String> {
    imp::cp(src, dst, recursive)
}

pub fn mv(src: &str, dst: &str) -> Result<(), String> {
    imp::mv(src, dst)
}

pub fn stat(path: &str) -> Result<Metadata, String> {
    imp::stat(path)
}

pub fn exists(path: &str) -> Result<bool, String> {
    imp::exists(path)
}
