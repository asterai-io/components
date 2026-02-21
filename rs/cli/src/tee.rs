use std::fs;

struct Opts {
    append: bool,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Opts {
    let mut opts = Opts {
        append: false,
        paths: Vec::new(),
    };
    for token in args.split_whitespace() {
        if token == "-a" {
            opts.append = true;
        } else {
            opts.paths.push(token.to_string());
        }
    }
    opts
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let input = stdin.unwrap_or_default();
    for path in &opts.paths {
        if opts.append {
            let existing = fs::read_to_string(path).unwrap_or_default();
            fs::write(path, format!("{existing}{input}"))
                .map_err(|e| format!("tee: {path}: {e}"))?;
        } else {
            fs::write(path, &input)
                .map_err(|e| format!("tee: {path}: {e}"))?;
        }
    }
    Ok(input)
}
