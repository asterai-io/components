use std::fs;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let mut parents = false;
    let mut paths = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'p' => parents = true,
                    _ => {}
                }
            }
        } else {
            paths.push(token);
        }
    }
    if paths.is_empty() {
        return Err("mkdir: missing operand".into());
    }
    for path in &paths {
        if parents {
            fs::create_dir_all(path).map_err(|e| format!("mkdir: {path}: {e}"))?;
        } else {
            fs::create_dir(path).map_err(|e| format!("mkdir: {path}: {e}"))?;
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
    fn create_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("new");
        cmd(sub.to_str().unwrap()).unwrap();
        assert!(sub.is_dir());
    }

    #[test]
    fn create_multiple() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        cmd(&format!("{} {}", a.display(), b.display())).unwrap();
        assert!(a.is_dir());
        assert!(b.is_dir());
    }

    #[test]
    fn parents_flag() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a/b/c");
        cmd(&format!("-p {}", deep.display())).unwrap();
        assert!(deep.is_dir());
    }

    #[test]
    fn no_parents_fails() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a/b/c");
        let err = cmd(deep.to_str().unwrap()).unwrap_err();
        assert!(err.contains("mkdir:"));
    }

    #[test]
    fn already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let err = cmd(dir.path().to_str().unwrap()).unwrap_err();
        assert!(err.contains("mkdir:"));
    }

    #[test]
    fn parents_existing_ok() {
        let dir = tempfile::tempdir().unwrap();
        cmd(&format!("-p {}", dir.path().display())).unwrap(); // no error
    }

    #[test]
    fn missing_operand() {
        let err = cmd("").unwrap_err();
        assert!(err.contains("missing operand"));
    }
}
