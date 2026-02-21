use std::fs;
use std::path::Path;
use std::time::SystemTime;

struct Opts {
    long: bool,
    all: bool,
    recursive: bool,
    sort_size: bool,
    sort_time: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        long: false,
        all: false,
        recursive: false,
        sort_size: false,
        sort_time: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'l' => opts.long = true,
                    'a' => opts.all = true,
                    'R' => opts.recursive = true,
                    'S' => opts.sort_size = true,
                    't' => opts.sort_time = true,
                    '1' => {} // one-per-line is the default
                    _ => {}
                }
            }
        } else {
            opts.paths.push(token.to_string());
        }
    }
    if opts.paths.is_empty() {
        opts.paths.push(".".into());
    }
    opts
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let mut output = String::new();
    let multi = opts.paths.len() > 1 || opts.recursive;

    for (i, path) in opts.paths.iter().enumerate() {
        if multi {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&format!("{path}:\n"));
        }
        list_dir(Path::new(path), &opts, &mut output, multi)?;
    }
    Ok(output)
}

struct EntryInfo {
    name: String,
    size: u64,
    modified: u64,
    is_dir: bool,
    mode: u32,
}

fn list_dir(dir: &Path, opts: &Opts, output: &mut String, _multi: bool) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("ls: {}: {e}", dir.display()))?;
    let mut items: Vec<EntryInfo> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !opts.all && name.starts_with('.') {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode()
        };
        #[cfg(not(unix))]
        let mode = 0o755u32;
        items.push(EntryInfo {
            name,
            size: meta.len(),
            modified,
            is_dir: meta.is_dir(),
            mode,
        });
    }

    if opts.sort_size {
        items.sort_by(|a, b| b.size.cmp(&a.size));
    } else if opts.sort_time {
        items.sort_by(|a, b| b.modified.cmp(&a.modified));
    } else {
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    for item in &items {
        if opts.long {
            let kind = if item.is_dir { 'd' } else { '-' };
            let perms = format_permissions(item.mode);
            let date = format_date(item.modified);
            output.push_str(&format!(
                "{kind}{perms} {:>8}  {date}  {}\n",
                item.size, item.name
            ));
        } else {
            output.push_str(&item.name);
            output.push('\n');
        }
    }

    if opts.recursive {
        for item in &items {
            if item.is_dir {
                let child = dir.join(&item.name);
                output.push_str(&format!("\n{}:\n", child.display()));
                list_dir(&child, opts, output, true)?;
            }
        }
    }

    Ok(())
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

    let months = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mon = months.get(m as usize).unwrap_or(&"???");
    format!("{mon} {d:2} {hour:02}:{minute:02} {y}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("alpha.txt"), "aaa").unwrap();
        fs::write(dir.path().join("beta.txt"), "bb").unwrap();
        fs::write(dir.path().join(".hidden"), "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        dir
    }

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn basic_listing() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.contains("alpha.txt"));
        assert!(out.contains("beta.txt"));
        assert!(out.contains("sub"));
    }

    #[test]
    fn hides_dotfiles_by_default() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(!out.contains(".hidden"));
    }

    #[test]
    fn show_all() {
        let dir = setup();
        let out = cmd(&format!("-a {}", dir.path().display())).unwrap();
        assert!(out.contains(".hidden"));
    }

    #[test]
    fn long_format() {
        let dir = setup();
        let out = cmd(&format!("-l {}", dir.path().display())).unwrap();
        assert!(out.contains("rw"));
        assert!(out.contains("alpha.txt"));
    }

    #[test]
    fn sorted_alphabetically() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        let alpha_pos = lines.iter().position(|l| l.contains("alpha")).unwrap();
        let beta_pos = lines.iter().position(|l| l.contains("beta")).unwrap();
        assert!(alpha_pos < beta_pos);
    }

    #[test]
    fn sort_by_size() {
        let dir = setup();
        let out = cmd(&format!("-lS {}", dir.path().display())).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        let alpha_pos = lines.iter().position(|l| l.contains("alpha")).unwrap();
        let beta_pos = lines.iter().position(|l| l.contains("beta")).unwrap();
        assert!(alpha_pos < beta_pos); // alpha (3 bytes) > beta (2 bytes)
    }

    #[test]
    fn recursive() {
        let dir = setup();
        fs::write(dir.path().join("sub/child.txt"), "").unwrap();
        let out = cmd(&format!("-R {}", dir.path().display())).unwrap();
        assert!(out.contains("child.txt"));
        assert!(out.contains("sub:"));
    }

    #[test]
    fn missing_dir() {
        let err = cmd("/no/such/dir").unwrap_err();
        assert!(err.contains("ls:"));
    }

    #[test]
    fn empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert_eq!(out, "");
    }
}
