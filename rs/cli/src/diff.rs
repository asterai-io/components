use std::fs;

struct Opts {
    unified: bool,
    context_lines: usize,
    file_a: String,
    file_b: String,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        unified: false,
        context_lines: 3,
        file_a: String::new(),
        file_b: String::new(),
    };
    let mut positional = Vec::new();
    let tokens: Vec<&str> = args.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "-u" {
            opts.unified = true;
        } else if tokens[i].starts_with("-U") {
            opts.unified = true;
            let n = &tokens[i][2..];
            if n.is_empty() {
                i += 1;
                opts.context_lines = tokens
                    .get(i)
                    .ok_or("diff: missing argument to -U")?
                    .parse()
                    .map_err(|_| "diff: invalid number for -U")?;
            } else {
                opts.context_lines = n.parse().map_err(|_| "diff: invalid number for -U")?;
            }
        } else {
            positional.push(tokens[i].to_string());
        }
        i += 1;
    }
    if positional.len() != 2 {
        return Err("diff: requires exactly two files".into());
    }
    opts.file_a = positional.remove(0);
    opts.file_b = positional.remove(0);
    Ok(opts)
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let a = fs::read_to_string(&opts.file_a)
        .map_err(|e| format!("diff: {}: {e}", opts.file_a))?;
    let b = fs::read_to_string(&opts.file_b)
        .map_err(|e| format!("diff: {}: {e}", opts.file_b))?;
    let lines_a: Vec<&str> = a.lines().collect();
    let lines_b: Vec<&str> = b.lines().collect();
    let edit = myers_diff(&lines_a, &lines_b);

    if edit.iter().all(|e| matches!(e, Edit::Equal(_))) {
        return Ok(String::new());
    }

    if opts.unified {
        Ok(format_unified(&opts, &edit))
    } else {
        Ok(format_normal(&edit))
    }
}

#[derive(Clone)]
enum Edit {
    Equal(String),
    Delete(String),
    Insert(String),
}

fn myers_diff(a: &[&str], b: &[&str]) -> Vec<Edit> {
    let n = a.len();
    let m = b.len();
    let max = n + m;
    if max == 0 {
        return Vec::new();
    }
    let mut v: Vec<isize> = vec![0; 2 * max + 1];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let offset = max as isize;

    'outer: for d in 0..=(max as isize) {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let idx = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };
            let mut y = x - k;
            while (x as usize) < n && (y as usize) < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x as usize >= n && y as usize >= m {
                break 'outer;
            }
            k += 2;
        }
    }

    // backtrack
    let mut edits = Vec::new();
    let mut x = n as isize;
    let mut y = m as isize;
    for d in (0..trace.len()).rev() {
        let v = &trace[d];
        let d = d as isize;
        let k = x - y;
        let prev_k = if k == -d || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize]) {
            k + 1
        } else {
            k - 1
        };
        let prev_x = v[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            edits.push(Edit::Equal(a[x as usize].to_string()));
        }
        if d > 0 {
            if x == prev_x {
                edits.push(Edit::Insert(b[(y - 1) as usize].to_string()));
                y -= 1;
            } else {
                edits.push(Edit::Delete(a[(x - 1) as usize].to_string()));
                x -= 1;
            }
        }
    }
    edits.reverse();
    edits
}

fn format_normal(edits: &[Edit]) -> String {
    let mut output = String::new();
    let mut a_line = 1usize;
    let mut b_line = 1usize;
    let mut i = 0;
    while i < edits.len() {
        match &edits[i] {
            Edit::Equal(_) => {
                a_line += 1;
                b_line += 1;
                i += 1;
            }
            Edit::Delete(_) => {
                let start_a = a_line;
                let mut dels = Vec::new();
                while i < edits.len() {
                    if let Edit::Delete(l) = &edits[i] {
                        dels.push(l.as_str());
                        a_line += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }
                // check for change (delete + insert)
                let mut ins = Vec::new();
                let start_b = b_line;
                while i < edits.len() {
                    if let Edit::Insert(l) = &edits[i] {
                        ins.push(l.as_str());
                        b_line += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }
                let a_range = range_str(start_a, a_line - 1);
                if ins.is_empty() {
                    output.push_str(&format!("{}d{}\n", a_range, b_line - 1));
                    for l in &dels {
                        output.push_str(&format!("< {l}\n"));
                    }
                } else {
                    let b_range = range_str(start_b, b_line - 1);
                    output.push_str(&format!("{a_range}c{b_range}\n"));
                    for l in &dels {
                        output.push_str(&format!("< {l}\n"));
                    }
                    output.push_str("---\n");
                    for l in &ins {
                        output.push_str(&format!("> {l}\n"));
                    }
                }
            }
            Edit::Insert(_) => {
                let start_b = b_line;
                let mut ins = Vec::new();
                while i < edits.len() {
                    if let Edit::Insert(l) = &edits[i] {
                        ins.push(l.as_str());
                        b_line += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }
                let b_range = range_str(start_b, b_line - 1);
                output.push_str(&format!("{}a{b_range}\n", a_line - 1));
                for l in &ins {
                    output.push_str(&format!("> {l}\n"));
                }
            }
        }
    }
    output
}

fn range_str(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start},{end}")
    }
}

#[cfg(test)]
fn run_with_strings(args: &str, a: &str, b: &str) -> Result<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let pa = dir.path().join("a.txt");
    let pb = dir.path().join("b.txt");
    std::fs::write(&pa, a).unwrap();
    std::fs::write(&pb, b).unwrap();
    let full_args = format!("{args} {} {}", pa.display(), pb.display());
    run(&full_args, None)
}

fn format_unified(opts: &Opts, edits: &[Edit]) -> String {
    let mut output = String::new();
    output.push_str(&format!("--- {}\n", opts.file_a));
    output.push_str(&format!("+++ {}\n", opts.file_b));

    // collect hunks
    let ctx = opts.context_lines;
    let mut hunks: Vec<(usize, usize)> = Vec::new(); // (start, end) indices into edits
    let mut i = 0;
    while i < edits.len() {
        if !matches!(&edits[i], Edit::Equal(_)) {
            let start = i.saturating_sub(ctx);
            let mut end = i;
            // extend through nearby changes
            loop {
                // skip current change block
                while end < edits.len() && !matches!(&edits[end], Edit::Equal(_)) {
                    end += 1;
                }
                // count following equal lines
                let eq_start = end;
                while end < edits.len() && matches!(&edits[end], Edit::Equal(_)) {
                    end += 1;
                }
                let eq_count = end - eq_start;
                if end < edits.len() && eq_count <= ctx * 2 {
                    // merge with next change
                    continue;
                }
                // trim trailing context
                end = std::cmp::min(eq_start + ctx, end);
                break;
            }
            hunks.push((start, end));
            i = end;
        } else {
            i += 1;
        }
    }

    for (start, end) in hunks {
        let mut a_start = 1usize;
        let mut b_start = 1usize;
        for e in &edits[..start] {
            match e {
                Edit::Equal(_) | Edit::Delete(_) => a_start += 1,
                Edit::Insert(_) => b_start += 1,
            }
        }
        let mut a_count = 0usize;
        let mut b_count = 0usize;
        let mut lines = Vec::new();
        for e in &edits[start..end] {
            match e {
                Edit::Equal(l) => {
                    lines.push(format!(" {l}"));
                    a_count += 1;
                    b_count += 1;
                }
                Edit::Delete(l) => {
                    lines.push(format!("-{l}"));
                    a_count += 1;
                }
                Edit::Insert(l) => {
                    lines.push(format!("+{l}"));
                    b_count += 1;
                }
            }
        }
        output.push_str(&format!("@@ -{a_start},{a_count} +{b_start},{b_count} @@\n"));
        for l in &lines {
            output.push_str(l);
            output.push('\n');
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(a: &str, b: &str) -> Result<String, String> {
        run_with_strings("", a, b)
    }

    fn diff_unified(a: &str, b: &str) -> Result<String, String> {
        run_with_strings("-u", a, b)
    }

    #[test]
    fn identical_files() {
        let out = diff("hello\nworld\n", "hello\nworld\n").unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn single_line_change() {
        let out = diff("aaa\n", "bbb\n").unwrap();
        assert!(out.contains("1c1"));
        assert!(out.contains("< aaa"));
        assert!(out.contains("> bbb"));
    }

    #[test]
    fn line_added() {
        let out = diff("aaa\n", "aaa\nbbb\n").unwrap();
        assert!(out.contains("a"));
        assert!(out.contains("> bbb"));
    }

    #[test]
    fn line_deleted() {
        let out = diff("aaa\nbbb\n", "aaa\n").unwrap();
        assert!(out.contains("d"));
        assert!(out.contains("< bbb"));
    }

    #[test]
    fn multiple_changes() {
        let out = diff("a\nb\nc\nd\n", "a\nB\nc\nD\n").unwrap();
        assert!(out.contains("< b"));
        assert!(out.contains("> B"));
        assert!(out.contains("< d"));
        assert!(out.contains("> D"));
    }

    #[test]
    fn unified_header() {
        let out = run_with_strings("-u", "a\n", "b\n").unwrap();
        assert!(out.contains("---"));
        assert!(out.contains("+++"));
        assert!(out.contains("@@"));
    }

    #[test]
    fn unified_context_lines() {
        let a = "1\n2\n3\n4\n5\n";
        let b = "1\n2\nX\n4\n5\n";
        let out = diff_unified(a, b).unwrap();
        assert!(out.contains("-3"));
        assert!(out.contains("+X"));
        assert!(out.contains(" 2"));
        assert!(out.contains(" 4"));
    }

    #[test]
    fn unified_custom_context() {
        let a = "1\n2\n3\n4\n5\n6\n7\n";
        let b = "1\n2\n3\nX\n5\n6\n7\n";
        let out = run_with_strings("-U1", a, b).unwrap();
        assert!(out.contains(" 3"));
        assert!(out.contains(" 5"));
        assert!(!out.contains(" 1\n"));
        assert!(!out.contains(" 7\n"));
    }

    #[test]
    fn both_empty() {
        let out = diff("", "").unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn empty_to_content() {
        let out = diff("", "hello\n").unwrap();
        assert!(out.contains("> hello"));
    }

    #[test]
    fn content_to_empty() {
        let out = diff("hello\n", "").unwrap();
        assert!(out.contains("< hello"));
    }

    #[test]
    fn missing_file() {
        let err = run("/no/a.txt /no/b.txt", None).unwrap_err();
        assert!(err.contains("diff:"));
    }

    #[test]
    fn requires_two_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        std::fs::write(&p, "x").unwrap();
        let err = run(&format!("{}", p.display()), None).unwrap_err();
        assert!(err.contains("requires exactly two files"));
    }

    #[test]
    fn normal_format_separator() {
        let out = diff("old\n", "new\n").unwrap();
        assert!(out.contains("---\n"));
    }
}
