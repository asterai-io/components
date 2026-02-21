use std::fs;
use std::path::Path;

struct Opts {
    recursive: bool,
    src: Vec<String>,
    dst: String,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut recursive = false;
    let mut paths = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'r' | 'R' => recursive = true,
                    _ => {}
                }
            }
        } else {
            paths.push(token.to_string());
        }
    }
    if paths.len() < 2 {
        return Err("cp: missing operand".into());
    }
    let dst = paths.pop().unwrap();
    Ok(Opts {
        recursive,
        src: paths,
        dst,
    })
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let dst = Path::new(&opts.dst);
    let multi = opts.src.len() > 1;

    if multi && !dst.is_dir() {
        return Err("cp: target is not a directory".into());
    }

    for src in &opts.src {
        let s = Path::new(src);
        let target = if dst.is_dir() {
            dst.join(s.file_name().ok_or_else(|| format!("cp: invalid source: {src}"))?)
        } else {
            dst.to_path_buf()
        };

        let meta = fs::metadata(s).map_err(|e| format!("cp: {src}: {e}"))?;
        if meta.is_dir() {
            if !opts.recursive {
                return Err(format!("cp: -r not specified; omitting directory '{src}'"));
            }
            cp_recursive(s, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
            }
            fs::copy(s, &target).map_err(|e| format!("cp: {src}: {e}"))?;
        }
    }
    Ok(String::new())
}

fn cp_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("cp: {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("cp: {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            cp_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(&entry.path(), &target)
                .map_err(|e| format!("cp: {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}
