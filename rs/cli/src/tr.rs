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
