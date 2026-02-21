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
