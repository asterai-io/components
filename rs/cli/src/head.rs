use std::fs;

struct Opts {
    lines: usize,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        lines: 10,
        paths: Vec::new(),
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "-n" {
            i += 1;
            if let Some(n) = tokens.get(i) {
                opts.lines = n.parse().unwrap_or(10);
            }
        } else if tokens[i].starts_with('-') && tokens[i][1..].chars().all(|c| c.is_ascii_digit()) {
            opts.lines = tokens[i][1..].parse().unwrap_or(10);
        } else {
            opts.paths.push(tokens[i].to_string());
        }
        i += 1;
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let mut output = String::new();
    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        let taken: Vec<&str> = input.lines().take(opts.lines).collect();
        output.push_str(&taken.join("\n"));
        if !taken.is_empty() {
            output.push('\n');
        }
    } else {
        let multi = opts.paths.len() > 1;
        for (i, path) in opts.paths.iter().enumerate() {
            if multi {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&format!("==> {path} <==\n"));
            }
            let content = fs::read_to_string(path)
                .map_err(|e| format!("head: {path}: {e}"))?;
            let taken: Vec<&str> = content.lines().take(opts.lines).collect();
            output.push_str(&taken.join("\n"));
            if !taken.is_empty() {
                output.push('\n');
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    #[test]
    fn default_ten_lines() {
        let input = (1..=20).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
        let out = cmd("", Some(&input)).unwrap();
        assert_eq!(out.lines().count(), 10);
        assert_eq!(out.lines().next().unwrap(), "1");
        assert_eq!(out.lines().last().unwrap(), "10");
    }

    #[test]
    fn custom_n_flag() {
        let out = cmd("-n 3", Some("a\nb\nc\nd\ne")).unwrap();
        assert_eq!(out, "a\nb\nc\n");
    }

    #[test]
    fn dash_n_shorthand() {
        let out = cmd("-5", Some("1\n2\n3\n4\n5\n6\n7")).unwrap();
        assert_eq!(out.lines().count(), 5);
    }

    #[test]
    fn fewer_lines_than_n() {
        let out = cmd("-n 100", Some("a\nb")).unwrap();
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn empty_input() {
        let out = cmd("", Some("")).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "x\ny\nz\n").unwrap();
        let out = cmd(&format!("-n 2 {}", p.display()), None).unwrap();
        assert_eq!(out, "x\ny\n");
    }

    #[test]
    fn multi_file_headers() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "aa\n").unwrap();
        std::fs::write(&p2, "bb\n").unwrap();
        let out = cmd(&format!("-n 1 {} {}", p1.display(), p2.display()), None).unwrap();
        assert!(out.contains("==> "));
        assert!(out.contains("a.txt"));
        assert!(out.contains("b.txt"));
    }

    #[test]
    fn missing_file() {
        let err = cmd("/no/file", None).unwrap_err();
        assert!(err.contains("head:"));
    }
}
