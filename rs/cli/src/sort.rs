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
