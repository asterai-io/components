use crate::fs_ops;
use std::path::Path;

struct Opts {
    recursive: bool,
    src: Vec<String>,
    dst: String,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut recursive = false;
    let mut paths = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'r' | 'R' => recursive = true,
                    _ => {}
                }
            }
        } else {
            paths.push(token.to_string());
        }
    }
    if paths.len() < 2 {
        return Err("cp: missing operand".into());
    }
    let dst = paths.pop().unwrap();
    Ok(Opts {
        recursive,
        src: paths,
        dst,
    })
}

pub fn run(args: &str, _stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let dst_is_dir = fs_ops::exists(&opts.dst)
        .unwrap_or(false)
        && fs_ops::stat(&opts.dst)
            .map(|m| m.is_dir())
            .unwrap_or(false);
    let multi = opts.src.len() > 1;

    if multi && !dst_is_dir {
        return Err("cp: target is not a directory".into());
    }

    for src in &opts.src {
        let src_meta = fs_ops::stat(src).map_err(|e| format!("cp: {src}: {e}"))?;

        if src_meta.is_dir() && !opts.recursive {
            return Err(format!("cp: -r not specified; omitting directory '{src}'"));
        }

        let target = if dst_is_dir {
            let name = Path::new(src)
                .file_name()
                .ok_or_else(|| format!("cp: invalid source: {src}"))?
                .to_string_lossy();
            format!("{}/{name}", opts.dst)
        } else {
            opts.dst.clone()
        };

        fs_ops::cp(src, &target, opts.recursive)
            .map_err(|e| format!("cp: {src}: {e}"))?;
    }
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str) -> Result<String, String> {
        run(args, None)
    }

    #[test]
    fn copy_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        std::fs::write(&src, "hello").unwrap();
        cmd(&format!("{} {}", src.display(), dst.display())).unwrap();
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello");
        assert!(src.exists());
    }

    #[test]
    fn copy_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let sub = dir.path().join("sub");
        std::fs::write(&src, "data").unwrap();
        std::fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {}", src.display(), sub.display())).unwrap();
        assert_eq!(std::fs::read_to_string(sub.join("a.txt")).unwrap(), "data");
    }

    #[test]
    fn copy_multiple_to_dir() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let sub = dir.path().join("out");
        std::fs::write(&a, "aa").unwrap();
        std::fs::write(&b, "bb").unwrap();
        std::fs::create_dir(&sub).unwrap();
        cmd(&format!("{} {} {}", a.display(), b.display(), sub.display())).unwrap();
        assert!(sub.join("a.txt").exists());
        assert!(sub.join("b.txt").exists());
    }

    #[test]
    fn recursive_dir() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dst = dir.path().join("dst_dir");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("f.txt"), "content").unwrap();
        cmd(&format!("-r {} {}", src.display(), dst.display())).unwrap();
        assert_eq!(std::fs::read_to_string(dst.join("f.txt")).unwrap(), "content");
    }

    #[test]
    fn dir_without_recursive_fails() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src_dir");
        let dst = dir.path().join("dst_dir");
        std::fs::create_dir(&src).unwrap();
        let err = cmd(&format!("{} {}", src.display(), dst.display())).unwrap_err();
        assert!(err.contains("-r not specified"));
    }

    #[test]
    fn missing_operand() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        std::fs::write(&p, "x").unwrap();
        let err = cmd(&format!("{}", p.display())).unwrap_err();
        assert!(err.contains("missing operand"));
    }

    #[test]
    fn missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let err = cmd(&format!("/no/file {}", dir.path().display())).unwrap_err();
        assert!(err.contains("cp:"));
    }
}
