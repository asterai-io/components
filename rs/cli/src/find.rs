use std::fs;
use std::path::Path;

struct Opts {
    root: String,
    name: Option<String>,
    file_type: Option<char>, // 'f' or 'd'
    maxdepth: Option<usize>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        root: ".".into(),
        name: None,
        file_type: None,
        maxdepth: None,
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    // first non-flag token is the root path
    if !tokens.is_empty() && !tokens[0].starts_with('-') {
        opts.root = tokens[0].to_string();
        i = 1;
    }
    while i < tokens.len() {
        match tokens[i] {
            "-name" => {
                i += 1;
                opts.name = Some(
                    tokens
                        .get(i)
                        .ok_or("find: missing argument to -name")?
                        .to_string(),
                );
            }
            "-type" => {
                i += 1;
                let t = *tokens.get(i).ok_or("find: missing argument to -type")?;
                match t {
                    "f" | "d" => opts.file_type = Some(t.chars().next().unwrap()),
                    _ => return Err(format!("find: unknown type: {t}")),
                }
            }
            "-maxdepth" => {
                i += 1;
                let n: usize = tokens
                    .get(i)
                    .ok_or("find: missing argument to -maxdepth")?
                    .parse()
                    .map_err(|_| "find: invalid number for -maxdepth")?;
                opts.maxdepth = Some(n);
            }
            _ => return Err(format!("find: unknown option: {}", tokens[i])),
        }
        i += 1;
    }
    Ok(opts)
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let mut output = Vec::new();
    walk(Path::new(&opts.root), &opts, 0, &mut output)?;
    Ok(output.join("\n") + if output.is_empty() { "" } else { "\n" })
}

fn glob_match(pattern: &str, name: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), name.as_bytes())
}

fn glob_match_inner(pat: &[u8], name: &[u8]) -> bool {
    let mut pi = 0;
    let mut ni = 0;
    let mut star_pi = usize::MAX;
    let mut star_ni = 0;

    while ni < name.len() {
        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == name[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < pat.len() && pat[pi] == b'*' {
            star_pi = pi;
            star_ni = ni;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ni += 1;
            ni = star_ni;
        } else {
            return false;
        }
    }
    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }
    pi == pat.len()
}

fn walk(
    dir: &Path,
    opts: &Opts,
    depth: usize,
    output: &mut Vec<String>,
) -> Result<(), String> {
    // check the root itself at depth 0
    if depth == 0 {
        if matches_entry(dir, opts)? {
            output.push(dir.display().to_string());
        }
    }

    if let Some(max) = opts.maxdepth {
        if depth >= max {
            return Ok(());
        }
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // skip unreadable dirs
    };

    let mut children: Vec<_> = entries
        .filter_map(|e| e.ok())
        .collect();
    children.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in children {
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if matches_entry_with_meta(&path, &meta, opts) {
            output.push(path.display().to_string());
        }

        if meta.is_dir() {
            walk(&path, opts, depth + 1, output)?;
        }
    }
    Ok(())
}

fn matches_entry(path: &Path, opts: &Opts) -> Result<bool, String> {
    let meta = fs::metadata(path).map_err(|e| format!("find: {}: {e}", path.display()))?;
    Ok(matches_entry_with_meta(path, &meta, opts))
}

fn matches_entry_with_meta(path: &Path, meta: &fs::Metadata, opts: &Opts) -> bool {
    if let Some(t) = opts.file_type {
        match t {
            'f' if !meta.is_file() => return false,
            'd' if !meta.is_dir() => return false,
            _ => {}
        }
    }
    if let Some(ref pattern) = opts.name {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        if !glob_match(pattern, &name) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.rs"), "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/c.txt"), "").unwrap();
        fs::create_dir(dir.path().join("sub/deep")).unwrap();
        fs::write(dir.path().join("sub/deep/d.txt"), "").unwrap();
        dir
    }

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    fn lines(out: &str) -> Vec<&str> {
        out.lines().collect()
    }

    #[test]
    fn find_all() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        let l = lines(&out);
        assert!(l.len() >= 6); // root + a.txt + b.rs + sub/ + sub/c.txt + sub/deep/ + sub/deep/d.txt
    }

    #[test]
    fn filter_by_name() {
        let dir = setup();
        let out = cmd(&format!("{} -name *.txt", dir.path().display())).unwrap();
        let l = lines(&out);
        assert!(l.iter().all(|p| p.ends_with(".txt")));
        assert_eq!(l.len(), 3); // a.txt, sub/c.txt, sub/deep/d.txt
    }

    #[test]
    fn filter_by_type_file() {
        let dir = setup();
        let out = cmd(&format!("{} -type f", dir.path().display())).unwrap();
        let l = lines(&out);
        for p in &l {
            assert!(Path::new(p).is_file());
        }
    }

    #[test]
    fn filter_by_type_dir() {
        let dir = setup();
        let out = cmd(&format!("{} -type d", dir.path().display())).unwrap();
        let l = lines(&out);
        for p in &l {
            assert!(Path::new(p).is_dir());
        }
        assert!(l.len() >= 3); // root, sub, sub/deep
    }

    #[test]
    fn maxdepth_zero() {
        let dir = setup();
        let out = cmd(&format!("{} -maxdepth 0", dir.path().display())).unwrap();
        let l = lines(&out);
        assert_eq!(l.len(), 1); // root only
    }

    #[test]
    fn maxdepth_one() {
        let dir = setup();
        let out = cmd(&format!("{} -maxdepth 1", dir.path().display())).unwrap();
        let l = lines(&out);
        // root + a.txt + b.rs + sub (no sub/c.txt, no sub/deep)
        assert!(!l.iter().any(|p| p.contains("deep")));
        assert!(!l.iter().any(|p| p.ends_with("c.txt")));
    }

    #[test]
    fn name_with_question_mark() {
        let dir = setup();
        let out = cmd(&format!("{} -name ?.txt", dir.path().display())).unwrap();
        let l = lines(&out);
        assert!(l.iter().all(|p| {
            let name = Path::new(p).file_name().unwrap().to_str().unwrap();
            name.len() == 5 && name.ends_with(".txt")
        }));
    }

    #[test]
    fn combined_name_and_type() {
        let dir = setup();
        let out = cmd(&format!("{} -name *.txt -type f", dir.path().display())).unwrap();
        let l = lines(&out);
        assert_eq!(l.len(), 3);
    }

    #[test]
    fn unknown_type() {
        let dir = setup();
        let err = cmd(&format!("{} -type x", dir.path().display())).unwrap_err();
        assert!(err.contains("unknown type"));
    }

    #[test]
    fn missing_root() {
        let err = cmd("/no/such/path").unwrap_err();
        assert!(err.contains("find:"));
    }

    #[test]
    fn empty_result() {
        let dir = setup();
        let out = cmd(&format!("{} -name *.zzz", dir.path().display())).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn glob_star() {
        assert!(glob_match("*.txt", "hello.txt"));
        assert!(!glob_match("*.txt", "hello.rs"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn glob_question() {
        assert!(glob_match("?.txt", "a.txt"));
        assert!(!glob_match("?.txt", "ab.txt"));
    }

    #[test]
    fn glob_exact() {
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));
    }
}
