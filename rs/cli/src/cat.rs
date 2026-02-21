use std::fs;

struct Opts {
    number: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        number: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'n' => opts.number = true,
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

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        return Ok(maybe_number(&input, opts.number));
    }

    let mut output = String::new();
    for path in &opts.paths {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cat: {path}: {e}"))?;
        output.push_str(&content);
    }
    Ok(maybe_number(&output, opts.number))
}

fn maybe_number(text: &str, number: bool) -> String {
    if !number {
        return text.to_string();
    }
    text.lines()
        .enumerate()
        .map(|(i, line)| format!("{:>6}\t{line}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}
