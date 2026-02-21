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
