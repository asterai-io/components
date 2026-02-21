use crate::fs_ops;

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
            let content = fs_ops::read_to_string(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: Option<&str>) -> Result<String, String> {
        run(args, stdin.map(String::from))
    }

    #[test]
    fn single_field_tab() {
        let out = cmd("-f1", Some("a\tb\tc\nd\te\tf")).unwrap();
        assert_eq!(out, "a\nd\n");
    }

    #[test]
    fn multiple_fields() {
        let out = cmd("-f1,3", Some("a\tb\tc")).unwrap();
        assert_eq!(out, "a\tc\n");
    }

    #[test]
    fn field_range() {
        let out = cmd("-f2-4", Some("a\tb\tc\td\te")).unwrap();
        assert_eq!(out, "b\tc\td\n");
    }

    #[test]
    fn custom_delimiter() {
        let out = cmd("-d , -f 2", Some("a,b,c")).unwrap();
        assert_eq!(out, "b\n");
    }

    #[test]
    fn inline_delimiter() {
        let out = cmd("-d: -f1", Some("root:x:0")).unwrap();
        assert_eq!(out, "root\n");
    }

    #[test]
    fn field_out_of_range() {
        let out = cmd("-f5", Some("a\tb")).unwrap();
        assert_eq!(out, "\n");
    }

    #[test]
    fn multiline() {
        let out = cmd("-d , -f 1,3", Some("a,b,c\nx,y,z")).unwrap();
        assert_eq!(out, "a,c\nx,z\n");
    }

    #[test]
    fn missing_fields() {
        let err = cmd("-d ,", Some("x")).unwrap_err();
        assert!(err.contains("must specify"));
    }

    #[test]
    fn invalid_field_spec() {
        let err = cmd("-f abc", Some("x")).unwrap_err();
        assert!(err.contains("invalid field"));
    }

    #[test]
    fn file_mode() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "a\tb\tc\n").unwrap();
        let out = cmd(&format!("-f2 {}", p.display()), None).unwrap();
        assert_eq!(out, "b\n");
    }
}
