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
