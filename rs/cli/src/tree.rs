use std::fs;

struct Opts {
    root: String,
    max_depth: Option<usize>,
    dirs_only: bool,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        root: ".".into(),
        max_depth: None,
        dirs_only: false,
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "-L" => {
                i += 1;
                if let Some(n) = tokens.get(i) {
                    opts.max_depth = Some(n.parse().unwrap_or(1));
                }
            }
            "-d" => opts.dirs_only = true,
            t if !t.starts_with('-') => opts.root = t.to_string(),
            _ => {}
        }
        i += 1;
    }
    opts
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let mut output = String::new();
    let mut dirs = 0usize;
    let mut files = 0usize;
    output.push_str(&opts.root);
    output.push('\n');
    walk(&opts.root, "", &opts, 0, &mut dirs, &mut files, &mut output)?;
    if opts.dirs_only {
        output.push_str(&format!("\n{dirs} directories\n"));
    } else {
        output.push_str(&format!("\n{dirs} directories, {files} files\n"));
    }
    Ok(output)
}

fn walk(
    dir: &str,
    prefix: &str,
    opts: &Opts,
    depth: usize,
    dirs: &mut usize,
    files: &mut usize,
    output: &mut String,
) -> Result<(), String> {
    if let Some(max) = opts.max_depth {
        if depth >= max {
            return Ok(());
        }
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("tree: {dir}: {e}"))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    // filter hidden files
    entries.retain(|e| {
        !e.file_name().to_string_lossy().starts_with('.')
    });

    if opts.dirs_only {
        entries.retain(|e| {
            e.metadata().map(|m| m.is_dir()).unwrap_or(false)
        });
    }

    let count = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251c}\u{2500}\u{2500} " };
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().map_err(|e| e.to_string())?;

        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&name);
        output.push('\n');

        if meta.is_dir() {
            *dirs += 1;
            let child_prefix = if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}\u{2502}   ")
            };
            let path = format!("{dir}/{name}");
            walk(&path, &child_prefix, opts, depth + 1, dirs, files, output)?;
        } else {
            *files += 1;
        }
    }
    Ok(())
}
