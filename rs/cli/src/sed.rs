use regex::Regex;
use std::fs;

struct Opts {
    in_place: bool,
    expression: String,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut in_place = false;
    let mut positional = Vec::new();
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if token == "-i" {
            in_place = true;
        } else if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'i' => in_place = true,
                    _ => {}
                }
            }
        } else {
            positional.push(token.to_string());
        }
        i += 1;
    }
    if positional.is_empty() {
        return Err("sed: missing expression".into());
    }
    let expression = positional.remove(0);
    Ok(Opts {
        in_place,
        expression,
        paths: positional,
    })
}

struct Sub {
    re: Regex,
    replacement: String,
    global: bool,
}

fn parse_sub(expr: &str) -> Result<Sub, String> {
    // s/pattern/replacement/flags
    if !expr.starts_with('s') || expr.len() < 4 {
        return Err(format!("sed: unsupported expression: {expr}"));
    }
    let delim = expr.as_bytes()[1] as char;
    let rest = &expr[2..];
    let parts: Vec<&str> = rest.splitn(3, delim).collect();
    if parts.len() < 2 {
        return Err(format!("sed: invalid substitution: {expr}"));
    }
    let pattern = parts[0];
    let replacement = parts[1];
    let flags = if parts.len() > 2 { parts[2] } else { "" };
    let global = flags.contains('g');
    let case_insensitive = flags.contains('i') || flags.contains('I');

    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .map_err(|e| format!("sed: invalid pattern: {e}"))?;

    Ok(Sub {
        re,
        replacement: replacement.to_string(),
        global,
    })
}

fn apply(text: &str, sub: &Sub) -> String {
    let mut output = String::new();
    for line in text.lines() {
        let replaced = if sub.global {
            sub.re.replace_all(line, sub.replacement.as_str())
        } else {
            sub.re.replace(line, sub.replacement.as_str())
        };
        output.push_str(&replaced);
        output.push('\n');
    }
    // Preserve missing trailing newline.
    if !text.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }
    output
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let sub = parse_sub(&opts.expression)?;

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        return Ok(apply(&input, &sub));
    }

    let mut output = String::new();
    for path in &opts.paths {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("sed: {path}: {e}"))?;
        let result = apply(&content, &sub);
        if opts.in_place {
            fs::write(path, &result)
                .map_err(|e| format!("sed: {path}: {e}"))?;
        } else {
            output.push_str(&result);
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
    fn basic_substitute_stdin() {
        let out = cmd("s/foo/bar/", Some("foo baz foo")).unwrap();
        assert_eq!(out, "bar baz foo");
    }

    #[test]
    fn global_flag() {
        let out = cmd("s/foo/bar/g", Some("foo baz foo")).unwrap();
        assert_eq!(out, "bar baz bar");
    }

    #[test]
    fn multiline() {
        let out = cmd("s/a/b/", Some("aaa\naaa")).unwrap();
        assert_eq!(out, "baa\nbaa");
    }

    #[test]
    fn global_multiline() {
        let out = cmd("s/a/b/g", Some("aaa\naaa")).unwrap();
        assert_eq!(out, "bbb\nbbb");
    }

    #[test]
    fn case_insensitive() {
        let out = cmd("s/hello/world/i", Some("Hello HELLO hello")).unwrap();
        assert_eq!(out, "world HELLO hello");
    }

    #[test]
    fn case_insensitive_global() {
        let out = cmd("s/hello/world/gi", Some("Hello HELLO hello")).unwrap();
        assert_eq!(out, "world world world");
    }

    #[test]
    fn custom_delimiter() {
        let out = cmd("s|foo|bar|", Some("foo")).unwrap();
        assert_eq!(out, "bar");
    }

    #[test]
    fn regex_pattern() {
        let out = cmd("s/[0-9]+/NUM/g", Some("abc 123 def 456")).unwrap();
        assert_eq!(out, "abc NUM def NUM");
    }

    #[test]
    fn preserves_trailing_newline() {
        let out = cmd("s/a/b/", Some("aaa\n")).unwrap();
        assert_eq!(out, "baa\n");
    }

    #[test]
    fn no_trailing_newline_preserved() {
        let out = cmd("s/a/b/", Some("aaa")).unwrap();
        assert_eq!(out, "baa");
    }

    #[test]
    fn empty_replacement() {
        let out = cmd("s/foo//g", Some("foo bar foo")).unwrap();
        assert_eq!(out, " bar ");
    }

    #[test]
    fn no_match() {
        let out = cmd("s/zzz/yyy/", Some("hello world")).unwrap();
        assert_eq!(out, "hello world");
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "hello world\n").unwrap();
        let out = cmd(&format!("s/world/rust/ {}", path.display()), None).unwrap();
        assert_eq!(out, "hello rust\n");
    }

    #[test]
    fn in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        std::fs::write(&path, "aaa\nbbb\n").unwrap();
        let out = cmd(&format!("-i s/aaa/ccc/ {}", path.display()), None).unwrap();
        assert_eq!(out, "");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "ccc\nbbb\n");
    }

    #[test]
    fn missing_expression() {
        let err = cmd("", Some("x")).unwrap_err();
        assert!(err.contains("missing expression"));
    }

    #[test]
    fn unsupported_expression() {
        let err = cmd("d", Some("x")).unwrap_err();
        assert!(err.contains("unsupported"));
    }

    #[test]
    fn invalid_regex() {
        let err = cmd("s/[bad/repl/", Some("x")).unwrap_err();
        assert!(err.contains("invalid pattern"));
    }

    #[test]
    fn capture_group_replacement() {
        let out = cmd("s/([a-z]+)@([a-z]+)/$2=$1/", Some("foo@bar")).unwrap();
        assert_eq!(out, "bar=foo");
    }
}
