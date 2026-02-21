use std::fs;

struct Opts {
    delimiter: char,
    fields: Vec<usize>,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        delimiter: '\t',
        fields: Vec::new(),
        paths: Vec::new(),
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "-d" => {
                i += 1;
                let d = tokens.get(i).ok_or("cut: missing argument to -d")?;
                opts.delimiter = d.chars().next().unwrap_or('\t');
            }
            "-f" => {
                i += 1;
                let spec = tokens.get(i).ok_or("cut: missing argument to -f")?;
                opts.fields = parse_fields(spec)?;
            }
            t if t.starts_with("-d") => {
                opts.delimiter = t[2..].chars().next().unwrap_or('\t');
            }
            t if t.starts_with("-f") => {
                opts.fields = parse_fields(&t[2..])?;
            }
            _ => opts.paths.push(tokens[i].to_string()),
        }
        i += 1;
    }
    if opts.fields.is_empty() {
        return Err("cut: you must specify a list of fields".into());
    }
    Ok(opts)
}

fn parse_fields(spec: &str) -> Result<Vec<usize>, String> {
    let mut fields = Vec::new();
    for part in spec.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            let start: usize = a.parse().map_err(|_| format!("cut: invalid field: {part}"))?;
            let end: usize = b.parse().map_err(|_| format!("cut: invalid field: {part}"))?;
            for f in start..=end {
                fields.push(f);
            }
        } else {
            let f: usize = part.parse().map_err(|_| format!("cut: invalid field: {part}"))?;
            fields.push(f);
        }
    }
    Ok(fields)
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let input = if opts.paths.is_empty() {
        stdin.unwrap_or_default()
    } else {
        let mut combined = String::new();
        for path in &opts.paths {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("cut: {path}: {e}"))?;
            combined.push_str(&content);
        }
        combined
    };

    let mut output = String::new();
    let delim = &opts.delimiter.to_string();
    for line in input.lines() {
        let parts: Vec<&str> = line.split(opts.delimiter).collect();
        let selected: Vec<&str> = opts
            .fields
            .iter()
            .filter_map(|&f| {
                if f >= 1 { parts.get(f - 1).copied() } else { None }
            })
            .collect();
        output.push_str(&selected.join(delim));
        output.push('\n');
    }
    Ok(output)
}
