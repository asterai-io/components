use crate::fs_ops;

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
    let mut entries = fs_ops::ls(dir, false)
        .map_err(|e| format!("tree: {dir}: {e}"))?;

    // filter hidden files
    entries.retain(|e| !e.name.starts_with('.'));

    if opts.dirs_only {
        entries.retain(|e| e.is_dir());
    }

    let count = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251c}\u{2500}\u{2500} " };

        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&entry.name);
        output.push('\n');

        if entry.is_dir() {
            *dirs += 1;
            let child_prefix = if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}\u{2502}   ")
            };
            let path = format!("{dir}/{}", entry.name);
            walk(&path, &child_prefix, opts, depth + 1, dirs, files, output)?;
        } else {
            *files += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/c.txt"), "").unwrap();
        dir
    }

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn basic_tree() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.contains("a.txt"));
        assert!(out.contains("b.txt"));
        assert!(out.contains("sub"));
        assert!(out.contains("c.txt"));
    }

    #[test]
    fn summary_line() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.contains("directories"));
        assert!(out.contains("files"));
    }

    #[test]
    fn dirs_only() {
        let dir = setup();
        let out = cmd(&format!("-d {}", dir.path().display())).unwrap();
        assert!(out.contains("sub"));
        assert!(!out.contains("a.txt"));
        assert!(!out.contains("b.txt"));
    }

    #[test]
    fn max_depth() {
        let dir = setup();
        let out = cmd(&format!("-L 1 {}", dir.path().display())).unwrap();
        assert!(out.contains("a.txt"));
        assert!(out.contains("sub"));
        assert!(!out.contains("c.txt"));
    }

    #[test]
    fn hides_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".hidden"), "").unwrap();
        std::fs::write(dir.path().join("visible"), "").unwrap();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(!out.contains(".hidden"));
        assert!(out.contains("visible"));
    }

    #[test]
    fn unicode_connectors() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.contains("├") || out.contains("└"));
    }

    #[test]
    fn root_header() {
        let dir = setup();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.starts_with(&dir.path().display().to_string()));
    }

    #[test]
    fn empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = cmd(&dir.path().display().to_string()).unwrap();
        assert!(out.contains("0 directories, 0 files"));
    }
}
