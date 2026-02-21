use std::fs;

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
        fs::read_to_string(path).map_err(|e| format!("uniq: {path}: {e}"))?
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
