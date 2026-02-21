use crate::fs_ops;

struct Opts {
    lines: bool,
    words: bool,
    chars: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        lines: false,
        words: false,
        chars: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'l' => opts.lines = true,
                    'w' => opts.words = true,
                    'c' | 'm' => opts.chars = true,
                    _ => {}
                }
            }
        } else {
            opts.paths.push(token.to_string());
        }
    }
    // default: show all
    if !opts.lines && !opts.words && !opts.chars {
        opts.lines = true;
        opts.words = true;
        opts.chars = true;
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let mut output = String::new();
    let mut total = (0usize, 0usize, 0usize);

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        let (l, w, c) = count(&input);
        format_line(&opts, l, w, c, None, &mut output);
    } else {
        for path in &opts.paths {
            let content = fs_ops::read_to_string(path)
                .map_err(|e| format!("wc: {path}: {e}"))?;
            let (l, w, c) = count(&content);
            total.0 += l;
            total.1 += w;
            total.2 += c;
            format_line(&opts, l, w, c, Some(path), &mut output);
        }
        if opts.paths.len() > 1 {
            format_line(&opts, total.0, total.1, total.2, Some("total"), &mut output);
        }
    }
    Ok(output)
}

fn count(text: &str) -> (usize, usize, usize) {
    let lines = text.lines().count();
    let words = text.split_whitespace().count();
    let chars = text.len();
    (lines, words, chars)
}

fn format_line(opts: &Opts, lines: usize, words: usize, chars: usize, name: Option<&str>, output: &mut String) {
    let mut parts = Vec::new();
    if opts.lines {
        parts.push(format!("{lines:>8}"));
    }
    if opts.words {
        parts.push(format!("{words:>8}"));
    }
    if opts.chars {
        parts.push(format!("{chars:>8}"));
    }
    output.push_str(&parts.join(""));
    if let Some(n) = name {
        output.push(' ');
        output.push_str(n);
    }
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    #[test]
    fn default_all_counts() {
        let out = cmd("", Some("hello world\nfoo\n")).unwrap();
        assert!(out.contains("2")); // 2 lines
        assert!(out.contains("3")); // 3 words
    }

    #[test]
    fn lines_only() {
        let out = cmd("-l", Some("a\nb\nc\n")).unwrap();
        assert!(out.trim().starts_with("3"));
    }

    #[test]
    fn words_only() {
        let out = cmd("-w", Some("one two three")).unwrap();
        assert!(out.trim().starts_with("3"));
    }

    #[test]
    fn chars_only() {
        let out = cmd("-c", Some("hello")).unwrap();
        assert!(out.trim().starts_with("5"));
    }

    #[test]
    fn combined_flags() {
        let out = cmd("-lw", Some("a b\nc d\n")).unwrap();
        assert!(out.contains("2")); // 2 lines
        assert!(out.contains("4")); // 4 words
    }

    #[test]
    fn empty_stdin() {
        let out = cmd("", Some("")).unwrap();
        assert!(out.contains("0"));
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "one two\nthree\n").unwrap();
        let out = cmd(&p.to_str().unwrap().to_string(), None).unwrap();
        assert!(out.contains("2")); // lines
        assert!(out.contains("3")); // words
        assert!(out.contains(&p.to_str().unwrap().to_string()));
    }

    #[test]
    fn multiple_files_total() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "one\n").unwrap();
        std::fs::write(&p2, "two\n").unwrap();
        let args = format!("{} {}", p1.display(), p2.display());
        let out = cmd(&args, None).unwrap();
        assert!(out.contains("total"));
    }

    #[test]
    fn missing_file() {
        let err = cmd("/no/file.txt", None).unwrap_err();
        assert!(err.contains("wc:"));
    }

    #[test]
    fn m_flag_same_as_c() {
        let out = cmd("-m", Some("hello")).unwrap();
        assert!(out.trim().starts_with("5"));
    }
}
