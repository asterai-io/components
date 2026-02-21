use hifijson::token::Lex;
use jaq_core::load::{Arena, File, Loader};
use jaq_core::{Compiler, Ctx, RcIter, ValT};
use jaq_json::Val;

struct Opts {
    raw_output: bool,
    compact: bool,
    filter: String,
}

fn parse_opts(args: &str) -> Opts {
    let mut raw_output = false;
    let mut compact = false;
    let mut positional = Vec::new();
    for token in args.split_whitespace() {
        if token.starts_with('-') && token.len() > 1 {
            for c in token[1..].chars() {
                match c {
                    'r' => raw_output = true,
                    'c' => compact = true,
                    _ => {}
                }
            }
        } else {
            positional.push(token.to_string());
        }
    }
    let filter = if positional.is_empty() {
        ".".to_string()
    } else {
        positional.join(" ")
    };
    Opts {
        raw_output,
        compact,
        filter,
    }
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args);
    let input = stdin.ok_or("jq: no input provided")?;

    // Parse JSON input via hifijson.
    let mut lexer = hifijson::SliceLexer::new(input.as_bytes());
    let val: Val = lexer
        .exactly_one(|token, lexer| Val::parse(token, lexer))
        .map_err(|e| format!("jq: invalid JSON: {e:?}"))?;

    // Compile jq filter.
    let program = File {
        code: opts.filter.as_str(),
        path: (),
    };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();
    let modules = loader
        .load(&arena, program)
        .map_err(|errs| format!("jq: parse error: {errs:?}"))?;
    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| format!("jq: compile error: {errs:?}"))?;

    let inputs = RcIter::new(core::iter::empty());
    let mut output = String::new();

    for result in filter.run((Ctx::new([], &inputs), val)) {
        match result {
            Ok(v) => {
                if opts.raw_output {
                    match v.as_str() {
                        Some(s) => output.push_str(s),
                        None => output.push_str(&v.to_string()),
                    }
                } else if opts.compact {
                    output.push_str(&v.to_string());
                } else {
                    // Pretty print: roundtrip through serde_json.
                    let json: serde_json::Value =
                        serde_json::from_str(&v.to_string()).unwrap_or(serde_json::Value::Null);
                    output.push_str(
                        &serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| v.to_string()),
                    );
                }
                output.push('\n');
            }
            Err(e) => return Err(format!("jq: {e}")),
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &str, stdin: &str) -> Result<String, String> {
        run(args, Some(stdin.to_string()))
    }

    #[test]
    fn identity() {
        let out = cmd(".", r#"{"a":1}"#).unwrap();
        assert!(out.contains("\"a\""));
        assert!(out.contains("1"));
    }

    #[test]
    fn field_access() {
        let out = cmd(".name", r#"{"name":"alice"}"#).unwrap();
        assert!(out.trim().contains("alice"));
    }

    #[test]
    fn nested_access() {
        let out = cmd(".a.b", r#"{"a":{"b":42}}"#).unwrap();
        assert_eq!(out.trim(), "42");
    }

    #[test]
    fn array_index() {
        let out = cmd(".[1]", r#"[10,20,30]"#).unwrap();
        assert_eq!(out.trim(), "20");
    }

    #[test]
    fn array_iterator() {
        let out = cmd(".[]", r#"[1,2,3]"#).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["1", "2", "3"]);
    }

    #[test]
    fn pipe() {
        let out = cmd(".items[] | .name", r#"{"items":[{"name":"a"},{"name":"b"}]}"#).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["\"a\"", "\"b\""]);
    }

    #[test]
    fn raw_output() {
        let out = cmd("-r .name", r#"{"name":"alice"}"#).unwrap();
        assert_eq!(out.trim(), "alice");
    }

    #[test]
    fn compact_output() {
        let out = cmd("-c .", r#"{"a": 1, "b": 2}"#).unwrap();
        assert!(!out.contains('\n') || out.trim_end().lines().count() == 1);
    }

    #[test]
    fn length() {
        let out = cmd("length", r#"[1,2,3]"#).unwrap();
        assert_eq!(out.trim(), "3");
    }

    #[test]
    fn keys() {
        let out = cmd("keys", r#"{"b":1,"a":2}"#).unwrap();
        assert!(out.contains("a"));
        assert!(out.contains("b"));
    }

    #[test]
    fn select() {
        let out = cmd(".[] | select(. > 2)", r#"[1,2,3,4]"#).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["3", "4"]);
    }

    #[test]
    fn no_input() {
        let err = run(".", None).unwrap_err();
        assert!(err.contains("no input"));
    }

    #[test]
    fn invalid_json() {
        let err = cmd(".", "not json").unwrap_err();
        assert!(err.contains("invalid JSON"));
    }

    #[test]
    fn invalid_filter() {
        let err = cmd(".[invalid", r#"{"a":1}"#).unwrap_err();
        assert!(err.contains("error"));
    }

    #[test]
    fn pretty_print_default() {
        let out = cmd(".", r#"{"a":1}"#).unwrap();
        assert!(out.contains('\n')); // pretty printed has newlines
    }
}
