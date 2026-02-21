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
        let all: Vec<&str> = input.lines().collect();
        let start = all.len().saturating_sub(opts.lines);
        let taken = &all[start..];
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
                .map_err(|e| format!("tail: {path}: {e}"))?;
            let all: Vec<&str> = content.lines().collect();
            let start = all.len().saturating_sub(opts.lines);
            let taken = &all[start..];
            output.push_str(&taken.join("\n"));
            if !taken.is_empty() {
                output.push('\n');
            }
        }
    }
    Ok(output)
}
