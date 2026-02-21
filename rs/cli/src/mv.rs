use std::fs;
use std::path::Path;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let mut paths: Vec<&str> = args.split_whitespace().collect();
    if paths.len() < 2 {
        return Err("mv: missing operand".into());
    }
    let dst = paths.pop().unwrap();
    let dst = Path::new(dst);

    if paths.len() > 1 && !dst.is_dir() {
        return Err("mv: target is not a directory".into());
    }

    for src in &paths {
        let s = Path::new(src);
        let target = if dst.is_dir() {
            dst.join(s.file_name().ok_or_else(|| format!("mv: invalid source: {src}"))?)
        } else {
            dst.to_path_buf()
        };
        fs::rename(s, &target).map_err(|e| format!("mv: {src}: {e}"))?;
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
    fn rename_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, "hello").unwrap();
        cmd(&format!("{} {}", src.display(), dst.display())).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
    }

    #[test]
    fn move_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let sub = dir.path().join("sub");
        fs::write(&src, "data").unwrap();
        fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {}", src.display(), sub.display())).unwrap();
        assert!(!src.exists());
        assert!(sub.join("a.txt").exists());
    }

    #[test]
    fn move_multiple_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let sub = dir.path().join("out");
        fs::write(&a, "aa").unwrap();
        fs::write(&b, "bb").unwrap();
        fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {} {}", a.display(), b.display(), sub.display())).unwrap();
        assert!(!a.exists());
        assert!(!b.exists());
        assert!(sub.join("a.txt").exists());
        assert!(sub.join("b.txt").exists());
    }

    #[test]
    fn missing_operand() {
        let err = cmd("onlyone").unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let err = cmd(&format!("/no/file {}", dir.path().display())).unwrap_err();
        assert!(err.contains("mv:"));
    }
}
