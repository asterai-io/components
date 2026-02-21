use std::fs;

struct Opts {
    append: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        append: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token == "-a" {
            opts.append = true;
        } else {
            opts.paths.push(token.to_string());
        }
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let input = stdin.unwrap_or_default();
    for path in &opts.paths {
        if opts.append {
            let existing = fs::read_to_string(path).unwrap_or_default();
            fs::write(path, format!("{existing}{input}"))
                .map_err(|e| format!("tee: {path}: {e}"))?;
        } else {
            fs::write(path, &input)
                .map_err(|e| format!("tee: {path}: {e}"))?;
        }
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: &str) -> Result<String, String> {
        run(args, Some(stdin.to_string()))
    }

    #[test]
    fn passthrough() {
        let out = cmd("", "hello").unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn write_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("out.txt");
        let out = cmd(p.to_str().unwrap(), "data").unwrap();
        assert_eq!(out, "data");
        assert_eq!(fs::read_to_string(&p).unwrap(), "data");
    }

    #[test]
    fn multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        let args = format!("{} {}", p1.display(), p2.display());
        cmd(&args, "content").unwrap();
        assert_eq!(fs::read_to_string(&p1).unwrap(), "content");
        assert_eq!(fs::read_to_string(&p2).unwrap(), "content");
    }

    #[test]
    fn append_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("out.txt");
        fs::write(&p, "old ").unwrap();
        cmd(&format!("-a {}", p.display()), "new").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "old new");
    }

    #[test]
    fn overwrite_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("out.txt");
        fs::write(&p, "old").unwrap();
        cmd(p.to_str().unwrap(), "new").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "new");
    }

    #[test]
    fn empty_stdin() {
        let out = cmd("", "").unwrap();
        assert_eq!(out, "");
    }
}
