use regex::RegexBuilder;
use std::fs;

struct Opts {
    ignore_case: bool,
    invert: bool,
    count: bool,
    line_number: bool,
    files_only: bool,
    recursive: bool,
    pattern: String,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        ignore_case: false,
        invert: false,
        count: false,
        line_number: false,
        files_only: false,
        recursive: false,
        pattern: String::new(),
        paths: Vec::new(),
    };
    let mut positional = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 && !token.starts_with("--") {
            for c in token[1..].chars() {
                match c {
                    'i' => opts.ignore_case = true,
                    'v' => opts.invert = true,
                    'c' => opts.count = true,
                    'n' => opts.line_number = true,
                    'l' => opts.files_only = true,
                    'r' | 'R' => opts.recursive = true,
                    _ => {}
                }
            }
        } else {
            positional.push(token.to_string());
        }
    }
    if positional.is_empty() {
        return Err("grep: missing pattern".into());
    }
    opts.pattern = positional.remove(0);
    opts.paths = positional;
    Ok(opts)
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let re = RegexBuilder::new(&opts.pattern)
        .case_insensitive(opts.ignore_case)
        .build()
        .map_err(|e| format!("grep: invalid pattern: {e}"))?;
    let mut output = String::new();

    if opts.paths.is_empty() {
        let input = stdin.unwrap_or_default();
        grep_lines(&input, None, &re, &opts, &mut output);
    } else {
        let mut files = Vec::new();
        for path in &opts.paths {
            if opts.recursive {
                collect_files(path, &mut files)?;
            } else {
                files.push(path.clone());
            }
        }
        let show_filename = files.len() > 1;
        for file in &files {
            let content = fs::read_to_string(file)
                .map_err(|e| format!("grep: {file}: {e}"))?;
            let label = if show_filename { Some(file.as_str()) } else { None };
            grep_lines(&content, label, &re, &opts, &mut output);
        }
    }
    Ok(output)
}

fn grep_lines(
    text: &str,
    filename: Option<&str>,
    re: &regex::Regex,
    opts: &Opts,
    output: &mut String,
) {
    let mut match_count = 0;
    for (i, line) in text.lines().enumerate() {
        let matched = re.is_match(line);
        let matched = if opts.invert { !matched } else { matched };

        if matched {
            if opts.files_only {
                if let Some(f) = filename {
                    output.push_str(f);
                } else {
                    output.push_str("(stdin)");
                }
                output.push('\n');
                return;
            }
            match_count += 1;
            if !opts.count {
                if let Some(f) = filename {
                    output.push_str(f);
                    output.push(':');
                }
                if opts.line_number {
                    output.push_str(&format!("{}:", i + 1));
                }
                output.push_str(line);
                output.push('\n');
            }
        }
    }
    if opts.count {
        if let Some(f) = filename {
            output.push_str(f);
            output.push(':');
        }
        output.push_str(&match_count.to_string());
        output.push('\n');
    }
}

fn collect_files(path: &str, files: &mut Vec<String>) -> Result<(), String> {
    let meta = fs::metadata(path).map_err(|e| format!("grep: {path}: {e}"))?;
    if meta.is_file() {
        files.push(path.to_string());
    } else if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| format!("grep: {path}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path().to_string_lossy().into_owned();
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            if meta.is_file() {
                files.push(p);
            } else if meta.is_dir() {
                collect_files(&p, files)?;
            }
        }
    }
    Ok(())
}
