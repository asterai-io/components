use std::fs;
use std::path::Path;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let mut recursive = false;
    let mut paths = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'r' | 'R' | 'f' => recursive = true,
                    _ => {}
                }
            }
        } else {
            paths.push(token);
        }
    }
    if paths.is_empty() {
        return Err("rm: missing operand".into());
    }
    for path in &paths {
        let p = Path::new(path);
        let meta = fs::metadata(p).map_err(|e| format!("rm: {path}: {e}"))?;
        if meta.is_dir() {
            if !recursive {
                return Err(format!("rm: {path}: is a directory"));
            }
            fs::remove_dir_all(p).map_err(|e| format!("rm: {path}: {e}"))?;
        } else {
            fs::remove_file(p).map_err(|e| format!("rm: {path}: {e}"))?;
        }
    }
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn remove_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        fs::write(&p, "x").unwrap();
        cmd(p.to_str().unwrap()).unwrap();
        assert!(!p.exists());
    }

    #[test]
    fn remove_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();
        cmd(&format!("{} {}", a.display(), b.display())).unwrap();
        assert!(!a.exists());
        assert!(!b.exists());
    }

    #[test]
    fn dir_without_recursive_fails() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let err = cmd(sub.to_str().unwrap()).unwrap_err();
        assert!(err.contains("is a directory"));
    }

    #[test]
    fn recursive_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("f.txt"), "").unwrap();
        cmd(&format!("-r {}", sub.display())).unwrap();
        assert!(!sub.exists());
    }

    #[test]
    fn missing_operand() {
        let err = cmd("").unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_file() {
        let err = cmd("/no/such/file").unwrap_err();
        assert!(err.contains("rm:"));
    }
}
