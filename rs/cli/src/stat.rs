use crate::fs_ops;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let paths: Vec<&str> = args.split_whitespace().collect();
    if paths.is_empty() {
        return Err("stat: missing operand".into());
    }
    let mut output = String::new();
    for path in &paths {
        let meta = fs_ops::stat(path).map_err(|e| format!("stat: {path}: {e}"))?;
        let file_type = if meta.is_dir() {
            "directory"
        } else {
            "regular file"
        };
        let modified = match meta.last_modified {
            Some(ts) => format_date(ts),
            None => "N/A".to_string(),
        };

        output.push_str(&format!("  File: {path}\n"));
        output.push_str(&format!("  Size: {:<14} {file_type}\n", meta.size));
        output.push_str(&format!("Modify: {modified}\n"));
    }
    Ok(output)
}

fn format_date(ts: u64) -> String {
    let secs = ts;
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y}-{m:02}-{d:02} {hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "hello").unwrap();
        let out = cmd(p.to_str().unwrap()).unwrap();
        assert!(out.contains("File:"));
        assert!(out.contains("regular file"));
        assert!(out.contains("Size: 5"));
    }

    #[test]
    fn directory() {
        let dir = tempfile::tempdir().unwrap();
        let out = cmd(dir.path().to_str().unwrap()).unwrap();
        assert!(out.contains("directory"));
    }

    #[test]
    fn shows_modify_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "x").unwrap();
        let out = cmd(p.to_str().unwrap()).unwrap();
        assert!(out.contains("Modify:"));
        assert!(out.contains("20"));
    }

    #[test]
    fn missing_operand() {
        let err = cmd("").unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_file() {
        let err = cmd("/no/such/file").unwrap_err();
        assert!(err.contains("stat:"));
    }

    #[test]
    fn multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "aa").unwrap();
        std::fs::write(&p2, "bbb").unwrap();
        let args = format!("{} {}", p1.display(), p2.display());
        let out = cmd(&args).unwrap();
        assert!(out.contains("Size: 2"));
        assert!(out.contains("Size: 3"));
    }
}
