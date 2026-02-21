struct Opts {
    delete: bool,
    set1: Vec<char>,
    set2: Vec<char>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        delete: false,
        set1: Vec::new(),
        set2: Vec::new(),
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut positional = Vec::new();
    for token in &tokens {
        if *token == "-d" {
            opts.delete = true;
        } else {
            positional.push(*token);
        }
    }
    if positional.is_empty() {
        return Err("tr: missing operand".into());
    }
    opts.set1 = expand_set(positional[0]);
    if positional.len() > 1 {
        opts.set2 = expand_set(positional[1]);
    } else if !opts.delete {
        return Err("tr: missing operand after set1".into());
    }
    Ok(opts)
}

fn expand_set(s: &str) -> Vec<char> {
    let mut chars = Vec::new();
    let raw: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < raw.len() {
        if i + 2 < raw.len() && raw[i + 1] == '-' {
            let start = raw[i];
            let end = raw[i + 2];
            for c in start..=end {
                chars.push(c);
            }
            i += 3;
        } else {
            chars.push(raw[i]);
            i += 1;
        }
    }
    chars
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let input = stdin.unwrap_or_default();
    let mut output = String::new();
    if opts.delete {
        for c in input.chars() {
            if !opts.set1.contains(&c) {
                output.push(c);
            }
        }
    } else {
        for c in input.chars() {
            if let Some(pos) = opts.set1.iter().position(|&s| s == c) {
                let replacement = if pos < opts.set2.len() {
                    opts.set2[pos]
                } else {
                    *opts.set2.last().unwrap_or(&c)
                };
                output.push(replacement);
            } else {
                output.push(c);
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: &str) -> Result<String, String> {
        run(args, Some(stdin.to_string()))
    }

    #[test]
    fn basic_translate() {
        let out = cmd("abc xyz", "aabbcc").unwrap();
        assert_eq!(out, "xxyyzz");
    }

    #[test]
    fn range_expansion() {
        let out = cmd("a-z A-Z", "hello").unwrap();
        assert_eq!(out, "HELLO");
    }

    #[test]
    fn delete_mode() {
        let out = cmd("-d aeiou", "hello world").unwrap();
        assert_eq!(out, "hll wrld");
    }

    #[test]
    fn set2_shorter_repeats_last() {
        let out = cmd("abc x", "aabbcc").unwrap();
        assert_eq!(out, "xxxxxx");
    }

    #[test]
    fn no_match_passthrough() {
        let out = cmd("x y", "hello").unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn single_char_sets() {
        let out = cmd("o 0", "foo").unwrap();
        assert_eq!(out, "f00");
    }

    #[test]
    fn missing_operand() {
        let err = run("", Some("x".into())).unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_set2() {
        let err = run("abc", Some("x".into())).unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn delete_no_set2_ok() {
        let out = cmd("-d x", "xaxbx").unwrap();
        assert_eq!(out, "ab");
    }

    #[test]
    fn preserves_newlines() {
        let out = cmd("a b", "a\na\n").unwrap();
        assert_eq!(out, "b\nb\n");
    }
}
