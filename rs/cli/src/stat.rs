use std::fs;
use std::time::SystemTime;

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let paths: Vec<&str> = args.split_whitespace().collect();
    if paths.is_empty() {
        return Err("stat: missing operand".into());
    }
    let mut output = String::new();
    for path in &paths {
        let meta = fs::metadata(path)
            .map_err(|e| format!("stat: {path}: {e}"))?;
        let file_type = if meta.is_dir() {
            "directory"
        } else if meta.is_symlink() {
            "symbolic link"
        } else {
            "regular file"
        };
        let size = meta.len();
        let modified = timestamp(&meta.modified());
        let accessed = timestamp(&meta.accessed());
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode()
        };
        #[cfg(not(unix))]
        let mode = 0o755u32;
        let perms = format_permissions(mode);
        let kind = if meta.is_dir() { 'd' } else { '-' };

        output.push_str(&format!("  File: {path}\n"));
        output.push_str(&format!("  Size: {size:<14} {file_type}\n"));
        output.push_str(&format!("Access: ({:04o}/{kind}{perms})\n", mode & 0o7777));
        output.push_str(&format!("Modify: {}\n", format_date(modified)));
        output.push_str(&format!("Access: {}\n", format_date(accessed)));
    }
    Ok(output)
}

fn timestamp(t: &std::io::Result<SystemTime>) -> u64 {
    t.as_ref()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }
    s
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
