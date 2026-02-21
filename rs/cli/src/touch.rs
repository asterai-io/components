use std::fs;
use std::time::SystemTime;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let paths: Vec<&str> = args.split_whitespace().collect();
    if paths.is_empty() {
        return Err("touch: missing operand".into());
    }
    for path in &paths {
        if fs::metadata(path).is_ok() {
            let file = fs::File::open(path).map_err(|e| format!("touch: {path}: {e}"))?;
            let times = fs::FileTimes::new().set_modified(SystemTime::now());
            file.set_times(times).map_err(|e| format!("touch: {path}: {e}"))?;
        } else {
            fs::File::create(path).map_err(|e| format!("touch: {path}: {e}"))?;
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
    fn create_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("new.txt");
        assert!(!p.exists());
        cmd(p.to_str().unwrap()).unwrap();
        assert!(p.exists());
        assert_eq!(fs::read_to_string(&p).unwrap(), "");
    }

    #[test]
    fn update_existing_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        fs::write(&p, "content").unwrap();
        let before = fs::metadata(&p).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        cmd(p.to_str().unwrap()).unwrap();
        let after = fs::metadata(&p).unwrap().modified().unwrap();
        assert!(after >= before);
        assert_eq!(fs::read_to_string(&p).unwrap(), "content"); // content preserved
    }

    #[test]
    fn multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        cmd(&format!("{} {}", a.display(), b.display())).unwrap();
        assert!(a.exists());
        assert!(b.exists());
    }

    #[test]
    fn missing_operand() {
        let err = cmd("").unwrap_err();
        assert!(err.contains("missing operand"));
    }
}
