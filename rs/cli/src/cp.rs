use std::fs;
use std::path::Path;

struct Opts {
    recursive: bool,
    src: Vec<String>,
    dst: String,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut recursive = false;
    let mut paths = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'r' | 'R' => recursive = true,
                    _ => {}
                }
            }
        } else {
            paths.push(token.to_string());
        }
    }
    if paths.len() < 2 {
        return Err("cp: missing operand".into());
    }
    let dst = paths.pop().unwrap();
    Ok(Opts {
        recursive,
        src: paths,
        dst,
    })
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let dst = Path::new(&opts.dst);
    let multi = opts.src.len() > 1;

    if multi && !dst.is_dir() {
        return Err("cp: target is not a directory".into());
    }

    for src in &opts.src {
        let s = Path::new(src);
        let target = if dst.is_dir() {
            dst.join(s.file_name().ok_or_else(|| format!("cp: invalid source: {src}"))?)
        } else {
            dst.to_path_buf()
        };

        let meta = fs::metadata(s).map_err(|e| format!("cp: {src}: {e}"))?;
        if meta.is_dir() {
            if !opts.recursive {
                return Err(format!("cp: -r not specified; omitting directory '{src}'"));
            }
            cp_recursive(s, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            fs::copy(s, &target).map_err(|e| format!("cp: {src}: {e}"))?;
        }
    }
    Ok(String::new())
}

fn cp_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("cp: {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("cp: {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            cp_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(&entry.path(), &target)
                .map_err(|e| format!("cp: {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn copy_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, "hello").unwrap();
        cmd(&format!("{} {}", src.display(), dst.display())).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
        assert!(src.exists()); // original still exists
    }

    #[test]
    fn copy_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let sub = dir.path().join("sub");
        fs::write(&src, "data").unwrap();
        fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {}", src.display(), sub.display())).unwrap();
        assert_eq!(fs::read_to_string(sub.join("a.txt")).unwrap(), "data");
    }

    #[test]
    fn copy_multiple_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let sub = dir.path().join("out");
        fs::write(&a, "aa").unwrap();
        fs::write(&b, "bb").unwrap();
        fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {} {}", a.display(), b.display(), sub.display())).unwrap();
        assert!(sub.join("a.txt").exists());
        assert!(sub.join("b.txt").exists());
    }

    #[test]
    fn recursive_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dst = dir.path().join("dst_dir");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("f.txt"), "content").unwrap();
        cmd(&format!("-r {} {}", src.display(), dst.display())).unwrap();
        assert_eq!(fs::read_to_string(dst.join("f.txt")).unwrap(), "content");
    }

    #[test]
    fn dir_without_recursive_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dst = dir.path().join("dst_dir");
        fs::create_dir(&src).unwrap();
        let err = cmd(&format!("{} {}", src.display(), dst.display())).unwrap_err();
        assert!(err.contains("-r not specified"));
    }

    #[test]
    fn missing_operand() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        fs::write(&p, "x").unwrap();
        let err = cmd(&format!("{}", p.display())).unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let err = cmd(&format!("/no/file {}", dir.path().display())).unwrap_err();
        assert!(err.contains("cp:"));
    }
}
