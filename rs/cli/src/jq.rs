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
