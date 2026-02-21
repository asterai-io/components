use crate::fs_ops;

struct Opts {
    number: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        number: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'n' => opts.number = true,
                    _ => {}
                }
            }
        } else {
            opts.paths.push(token.to_string());
        }
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        return Ok(maybe_number(&input, opts.number));
    }

    let mut output = String::new();
    for path in &opts.paths {
        let content = fs_ops::read_to_string(path)
            .map_err(|e| format!("cat: {path}: {e}"))?;
        output.push_str(&content);
    }
    Ok(maybe_number(&output, opts.number))
}

fn maybe_number(text: &str, number: bool) -> String {
    if !number {
        return text.to_string();
    }
    text.lines()
        .enumerate()
        .map(|(i, line)| format!("{:>6}\t{line}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    fn tmp(name: &str, content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        let p = path.to_str().unwrap().to_string();
        (dir, p)
    }

    #[test]
    fn stdin_passthrough() {
        assert_eq!(cmd("", Some("hello\nworld")).unwrap(), "hello\nworld");
    }

    #[test]
    fn stdin_empty() {
        assert_eq!(cmd("", None).unwrap(), "");
    }

    #[test]
    fn stdin_number_lines() {
        let out = cmd("-n", Some("aaa\nbbb\nccc")).unwrap();
        assert_eq!(out, "     1\taaa\n     2\tbbb\n     3\tccc");
    }

    #[test]
    fn read_file() {
        let (_dir, path) = tmp("hello.txt", "file content\n");
        assert_eq!(cmd(&path, None).unwrap(), "file content\n");
    }

    #[test]
    fn read_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "aaa\n").unwrap();
        std::fs::write(&p2, "bbb\n").unwrap();
        let args = format!("{} {}", p1.to_str().unwrap(), p2.to_str().unwrap());
        assert_eq!(cmd(&args, None).unwrap(), "aaa\nbbb\n");
    }

    #[test]
    fn file_with_number() {
        let (_dir, path) = tmp("f.txt", "x\ny\n");
        let out = cmd(&format!("-n {path}"), None).unwrap();
        assert_eq!(out, "     1\tx\n     2\ty");
    }

    #[test]
    fn missing_file() {
        let err = cmd("/no/such/file.txt", None).unwrap_err();
        assert!(err.contains("cat:"));
    }
}
