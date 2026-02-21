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
