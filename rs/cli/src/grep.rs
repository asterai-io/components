use regex::RegexBuilder;
use std::fs;

struct Opts {
    ignore_case: bool,
    invert: bool,
    count: bool,
    line_number: bool,
    files_only: bool,
    recursive: bool,
    pattern: String,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        ignore_case: false,
        invert: false,
        count: false,
        line_number: false,
        files_only: false,
        recursive: false,
        pattern: String::new(),
        paths: Vec::new(),
    };
    let mut positional = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'i' => opts.ignore_case = true,
                    'v' => opts.invert = true,
                    'c' => opts.count = true,
                    'n' => opts.line_number = true,
                    'l' => opts.files_only = true,
                    'r' | 'R' => opts.recursive = true,
                    _ => {}
                }
            }
        } else {
            positional.push(token.to_string());
        }
    }
    if positional.is_empty() {
        return Err("grep: missing pattern".into());
    }
    opts.pattern = positional.remove(0);
    opts.paths = positional;
    Ok(opts)
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let re = RegexBuilder::new(&opts.pattern)
        .case_insensitive(opts.ignore_case)
        .build()
        .map_err(|e| format!("grep: invalid pattern: {e}"))?;
    let mut output = String::new();

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        grep_lines(&input, None, &re, &opts, &mut output);
    } else {
        let mut files = Vec::new();
        for path in &opts.paths {
            if opts.recursive {
                collect_files(path, &mut files)?;
            } else {
                files.push(path.clone());
            }
        }
        let show_filename = files.len() > 1;
        for file in &files {
            let content = fs::read_to_string(file)
                .map_err(|e| format!("grep: {file}: {e}"))?;
            let label = if show_filename { Some(file.as_str()) } else { None };
            grep_lines(&content, label, &re, &opts, &mut output);
        }
    }
    Ok(output)
}

fn grep_lines(
    text: &str,
    filename: Option<&str>,
    re: &regex::Regex,
    opts: &Opts,
    output: &mut String,
) {
    let mut match_count = 0;
    for (i, line) in text.lines().enumerate() {
        let matched = re.is_match(line);
        let matched = if opts.invert { !matched } else { matched };

        if matched {
            if opts.files_only {
                if let Some(f) = filename {
                    output.push_str(f);
                } else {
                    output.push_str("(stdin)");
                }
                output.push('\n');
                return;
            }
            match_count += 1;
            if !opts.count {
                if let Some(f) = filename {
                    output.push_str(f);
                    output.push(':');
                }
                if opts.line_number {
                    output.push_str(&format!("{}:", i + 1));
                }
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    if opts.count {
        if let Some(f) = filename {
            output.push_str(f);
            output.push(':');
        }
        output.push_str(&match_count.to_string());
        output.push('\n');
    }
}

fn collect_files(path: &str, files: &mut Vec<String>) -> Result<(), String> {
    let meta = fs::metadata(path).map_err(|e| format!("grep: {path}: {e}"))?;
    if meta.is_file() {
        files.push(path.to_string());
    } else if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| format!("grep: {path}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path().to_string_lossy().into_owned();
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.is_file() {
                files.push(p);
            } else if meta.is_dir() {
                collect_files(&p, files)?;
            }
        }
    }
    Ok(())
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
        (dir, path.to_str().unwrap().to_string())
    }

    #[test]
    fn basic_match_stdin() {
        let out = cmd("hello", Some("hello world\nfoo\nhello again")).unwrap();
        assert_eq!(out, "hello world\nhello again\n");
    }

    #[test]
    fn no_match_stdin() {
        let out = cmd("zzz", Some("aaa\nbbb")).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn regex_pattern() {
        let out = cmd("^f.o$", Some("foo\nbar\nfzo")).unwrap();
        assert_eq!(out, "foo\nfzo\n");
    }

    #[test]
    fn case_insensitive() {
        let out = cmd("-i hello", Some("Hello\nHELLO\nworld")).unwrap();
        assert_eq!(out, "Hello\nHELLO\n");
    }

    #[test]
    fn invert_match() {
        let out = cmd("-v foo", Some("foo\nbar\nbaz")).unwrap();
        assert_eq!(out, "bar\nbaz\n");
    }

    #[test]
    fn count_flag() {
        let out = cmd("-c foo", Some("foo\nbar\nfoo baz")).unwrap();
        assert_eq!(out, "2\n");
    }

    #[test]
    fn line_numbers() {
        let out = cmd("-n bar", Some("foo\nbar\nbaz\nbar")).unwrap();
        assert_eq!(out, "2:bar\n4:bar\n");
    }

    #[test]
    fn files_only_stdin() {
        let out = cmd("-l match", Some("no\nmatch here")).unwrap();
        assert_eq!(out, "(stdin)\n");
    }

    #[test]
    fn files_only_no_match() {
        let out = cmd("-l zzz", Some("aaa\nbbb")).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn combined_flags() {
        let out = cmd("-inv hello", Some("hello\nworld\nHELLO")).unwrap();
        assert_eq!(out, "2:world\n");
    }

    #[test]
    fn missing_pattern() {
        let err = cmd("", Some("x")).unwrap_err();
        assert!(err.contains("missing pattern"));
    }

    #[test]
    fn invalid_regex() {
        let err = cmd("[invalid", Some("x")).unwrap_err();
        assert!(err.contains("invalid pattern"));
    }

    #[test]
    fn search_file() {
        let (_dir, path) = tmp("f.txt", "alpha\nbeta\ngamma\n");
        let out = cmd(&format!("beta {path}"), None).unwrap();
        assert_eq!(out, "beta\n");
    }

    #[test]
    fn search_file_count() {
        let (_dir, path) = tmp("f.txt", "aa\nab\nac\n");
        let out = cmd(&format!("-c ^a {path}"), None).unwrap();
        assert_eq!(out, "3\n");
    }

    #[test]
    fn multiple_files_show_filename() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "foo\nbar\n").unwrap();
        std::fs::write(&p2, "baz\nfoo\n").unwrap();
        let args = format!("foo {} {}", p1.display(), p2.display());
        let out = cmd(&args, None).unwrap();
        assert!(out.contains("a.txt:foo"));
        assert!(out.contains("b.txt:foo"));
    }

    #[test]
    fn files_only_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.txt");
        let p2 = dir.path().join("b.txt");
        std::fs::write(&p1, "match\n").unwrap();
        std::fs::write(&p2, "nope\n").unwrap();
        let args = format!("-l match {} {}", p1.display(), p2.display());
        let out = cmd(&args, None).unwrap();
        assert!(out.contains("a.txt"));
        assert!(!out.contains("b.txt"));
    }

    #[test]
    fn recursive_search() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(dir.path().join("top.txt"), "needle\n").unwrap();
        std::fs::write(sub.join("deep.txt"), "needle\nhay\n").unwrap();
        let args = format!("-r needle {}", dir.path().display());
        let out = cmd(&args, None).unwrap();
        assert!(out.contains("needle"));
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn missing_file() {
        let err = cmd("pat /no/such/file.txt", None).unwrap_err();
        assert!(err.contains("grep:"));
    }

    #[test]
    fn count_with_invert() {
        let out = cmd("-vc x", Some("x\na\nb\nx")).unwrap();
        assert_eq!(out, "2\n");
    }
}
