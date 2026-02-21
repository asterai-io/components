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
