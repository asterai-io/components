use crate::fs_ops;

struct Opts {
    count: bool,
    duplicates_only: bool,
    ignore_case: bool,
    path: Option<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        count: false,
        duplicates_only: false,
        ignore_case: false,
        path: None,
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'c' => opts.count = true,
                    'd' => opts.duplicates_only = true,
                    'i' => opts.ignore_case = true,
                    _ => {}
                }
            }
        } else if opts.path.is_none() {
            opts.path = Some(token.to_string());
        }
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let input = if let Some(path) = &opts.path {
        fs_ops::read_to_string(path).map_err(|e| format!("uniq: {path}: {e}"))?
    } else {
        stdin.unwrap_or_default()
    };

    let mut output = String::new();
    let mut prev: Option<String> = None;
    let mut count = 0usize;

    for line in input.lines() {
        let key = if opts.ignore_case {
            line.to_lowercase()
        } else {
            line.to_string()
        };
        if prev.as_deref() == Some(&key) {
            count += 1;
        } else {
            flush(&opts, &prev, count, &mut output);
            prev = Some(key);
            count = 1;
        }
    }
    flush(&opts, &prev, count, &mut output);
    Ok(output)
}

fn flush(opts: &Opts, prev: &Option<String>, count: usize, output: &mut String) {
    if let Some(line) = prev {
        if opts.duplicates_only && count < 2 {
            return;
        }
        if opts.count {
            output.push_str(&format!("{count:>7} {line}\n"));
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    #[test]
    fn basic_dedup() {
        let out = cmd("", Some("a\na\nb\nb\nb\nc")).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn non_adjacent_not_deduped() {
        let out = cmd("", Some("a\nb\na")).unwrap();
        assert_eq!(out, "a\nb\na\n");
    }

    #[test]
    fn count_flag() {
        let out = cmd("-c", Some("a\na\nb\nc\nc\nc")).unwrap();
        assert!(out.contains("2 a"));
        assert!(out.contains("1 b"));
        assert!(out.contains("3 c"));
    }

    #[test]
    fn duplicates_only() {
        let out = cmd("-d", Some("a\na\nb\nc\nc")).unwrap();
        assert_eq!(out, "a\nc\n");
    }

    #[test]
    fn case_insensitive() {
        let out = cmd("-i", Some("Hello\nhello\nHELLO\nworld")).unwrap();
        assert_eq!(out, "hello\nworld\n");
    }

    #[test]
    fn empty_input() {
        let out = cmd("", Some("")).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn no_duplicates() {
        let out = cmd("", Some("a\nb\nc")).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn count_with_duplicates_only() {
        let out = cmd("-cd", Some("a\na\nb\nc\nc")).unwrap();
        assert!(out.contains("2 a"));
        assert!(out.contains("2 c"));
        assert!(!out.contains("b"));
    }

    #[test]
    fn single_line() {
        let out = cmd("", Some("only")).unwrap();
        assert_eq!(out, "only\n");
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "x\nx\ny\n").unwrap();
        let out = cmd(&p.to_str().unwrap().to_string(), None).unwrap();
        assert_eq!(out, "x\ny\n");
    }
}
