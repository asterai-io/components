use crate::fs_ops;
use std::path::Path;

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

struct ItemInfo {
    name: String,
    size: u64,
    modified: Option<u64>,
    is_dir: bool,
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
        list_dir(path, &opts, &mut output, multi)?;
    }
    Ok(output)
}

fn list_dir(dir: &str, opts: &Opts, output: &mut String, _multi: bool) -> Result<(), String> {
    let entries = fs_ops::ls(dir, false).map_err(|e| format!("ls: {dir}: {e}"))?;
    let mut items: Vec<ItemInfo> = Vec::new();

    for entry in entries {
        if !opts.all && entry.name.starts_with('.') {
            continue;
        }
        let full_path = format!("{dir}/{}", entry.name);
        let modified = if opts.long || opts.sort_time {
            fs_ops::stat(&full_path).ok().and_then(|m| m.last_modified)
        } else {
            None
        };
        let is_dir = entry.is_dir();
        items.push(ItemInfo {
            name: entry.name,
            size: entry.size,
            modified,
            is_dir,
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
            let date = match item.modified {
                Some(ts) => format_date(ts),
                None => "               ".to_string(),
            };
            output.push_str(&format!(
                "{kind} {:>8}  {date}  {}\n",
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
                let child = Path::new(dir).join(&item.name);
                let child_str = child.to_string_lossy();
                output.push_str(&format!("\n{child_str}:\n"));
                list_dir(&child_str, opts, output, true)?;
            }
        }
    }

    Ok(())
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
        std::fs::write(dir.path().join("alpha.txt"), "aaa").unwrap();
        std::fs::write(dir.path().join("beta.txt"), "bb").unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
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
        assert!(out.contains("alpha.txt"));
        assert!(out.contains("3")); // size of "aaa"
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
        std::fs::write(dir.path().join("sub/child.txt"), "").unwrap();
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
