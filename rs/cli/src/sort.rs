use std::fs;

struct Opts {
    reverse: bool,
    numeric: bool,
    unique: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        reverse: false,
        numeric: false,
        unique: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'r' => opts.reverse = true,
                    'n' => opts.numeric = true,
                    'u' => opts.unique = true,
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
    let input = if opts.paths.is_empty() {
        stdin.unwrap_or_default()
    } else {
        let mut combined = String::new();
        for path in &opts.paths {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("sort: {path}: {e}"))?;
            combined.push_str(&content);
        }
        combined
    };

    let mut lines: Vec<&str> = input.lines().collect();

    if opts.numeric {
        lines.sort_by(|a, b| {
            let na = parse_num(a);
            let nb = parse_num(b);
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort();
    }

    if opts.reverse {
        lines.reverse();
    }

    if opts.unique {
        lines.dedup();
    }

    let mut output = lines.join("\n");
    if !lines.is_empty() {
        output.push('\n');
    }
    Ok(output)
}

fn parse_num(s: &str) -> f64 {
    s.trim()
        .chars()
        .take_while(|c| *c == '-' || *c == '.' || c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    #[test]
    fn alphabetical() {
        let out = cmd("", Some("banana\napple\ncherry")).unwrap();
        assert_eq!(out, "apple\nbanana\ncherry\n");
    }

    #[test]
    fn reverse() {
        let out = cmd("-r", Some("a\nb\nc")).unwrap();
        assert_eq!(out, "c\nb\na\n");
    }

    #[test]
    fn numeric() {
        let out = cmd("-n", Some("10\n2\n1\n20")).unwrap();
        assert_eq!(out, "1\n2\n10\n20\n");
    }

    #[test]
    fn numeric_reverse() {
        let out = cmd("-nr", Some("1\n3\n2")).unwrap();
        assert_eq!(out, "3\n2\n1\n");
    }

    #[test]
    fn unique() {
        let out = cmd("-u", Some("a\na\nb\nb\nc")).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn numeric_with_text() {
        let out = cmd("-n", Some("10apples\n2bananas\n1cherry")).unwrap();
        assert_eq!(out, "1cherry\n2bananas\n10apples\n");
    }

    #[test]
    fn empty_input() {
        let out = cmd("", Some("")).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_line() {
        let out = cmd("", Some("only")).unwrap();
        assert_eq!(out, "only\n");
    }

    #[test]
    fn already_sorted() {
        let out = cmd("", Some("a\nb\nc")).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "c\na\nb\n").unwrap();
        let out = cmd(&p.to_str().unwrap().to_string(), None).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }
}
