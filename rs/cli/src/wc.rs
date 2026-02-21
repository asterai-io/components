use std::fs;

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
            let content = fs::read_to_string(path)
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
