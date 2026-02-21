//! Partial awk implementation covering the common subset: field extraction, patterns,
//! BEGIN/END, arithmetic, variables, and regex matching. Unsupported features (loops,
//! arrays, functions, printf, etc.) return clear "not implemented" errors.
use std::fs;
use regex::Regex;

struct Opts {
    field_sep: String,
    program: String,
    paths: Vec<String>,
}

fn parse_opts(args: &str) -> Result<Opts, String> {
    let mut opts = Opts {
        field_sep: " ".into(),
        program: String::new(),
        paths: Vec::new(),
    };
    let tokens = shell_split(args)?;
    let mut i = 0;
    let mut got_program = false;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-F" => {
                i += 1;
                opts.field_sep = tokens.get(i).ok_or("awk: missing argument to -F")?.clone();
            }
            t if t.starts_with("-F") => {
                opts.field_sep = t[2..].to_string();
            }
            _ => {
                if !got_program {
                    opts.program = tokens[i].clone();
                    got_program = true;
                } else {
                    opts.paths.push(tokens[i].clone());
                }
            }
        }
        i += 1;
    }
    if !got_program {
        return Err("awk: missing program".into());
    }
    Ok(opts)
}

/// Simple shell-like splitting that handles single and double quotes.
fn shell_split(s: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if in_single {
            if c == '\'' {
                in_single = false;
            } else {
                current.push(c);
            }
        } else if in_double {
            if c == '"' {
                in_double = false;
            } else {
                current.push(c);
            }
        } else if c == '\'' {
            in_single = true;
        } else if c == '"' {
            in_double = true;
        } else if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if in_single || in_double {
        return Err("awk: unterminated quote".into());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

#[derive(Debug)]
struct Rule {
    pattern: Pattern,
    actions: Vec<Action>,
}

#[derive(Debug)]
enum Pattern {
    Begin,
    End,
    Always,
    Regex(String),
    Condition(Expr),
}

#[derive(Debug, Clone)]
enum Expr {
    Field(Box<Expr>),
    Literal(f64),
    StringLit(String),
    NR,
    NF,
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    Assign(String, Box<Expr>),
    AssignOp(String, BinOp, Box<Expr>),
    Match(Box<Expr>, String),
    NotMatch(Box<Expr>, String),
    Concat(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
}

#[derive(Debug)]
enum Action {
    Print(Vec<Expr>),
    Expr(Expr),
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let c = self.input.as_bytes().get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "awk: expected '{}', got '{}'",
                c as char,
                self.peek().map(|b| b as char).unwrap_or('\0')
            ))
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Rule>, String> {
        let mut rules = Vec::new();
        loop {
            self.skip_ws();
            if self.pos >= self.input.len() {
                break;
            }
            // skip semicolons between rules
            if self.peek() == Some(b';') {
                self.advance();
                continue;
            }
            rules.push(self.parse_rule()?);
        }
        Ok(rules)
    }

    fn parse_rule(&mut self) -> Result<Rule, String> {
        self.skip_ws();
        let pattern = self.parse_pattern()?;
        self.skip_ws();
        let actions = if self.peek() == Some(b'{') {
            self.parse_action_block()?
        } else {
            match &pattern {
                Pattern::Begin | Pattern::End => {
                    return Err("awk: BEGIN/END requires action block".into());
                }
                _ => vec![Action::Print(vec![Expr::Field(Box::new(Expr::Literal(0.0)))])],
            }
        };
        Ok(Rule { pattern, actions })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        self.skip_ws();
        if self.starts_with("BEGIN") && self.is_word_boundary(5) {
            self.pos += 5;
            return Ok(Pattern::Begin);
        }
        if self.starts_with("END") && self.is_word_boundary(3) {
            self.pos += 3;
            return Ok(Pattern::End);
        }
        if self.peek() == Some(b'{') {
            return Ok(Pattern::Always);
        }
        if self.peek() == Some(b'/') {
            let re = self.parse_regex_literal()?;
            return Ok(Pattern::Regex(re));
        }
        // try to parse a condition expression
        if self.pos < self.input.len() && self.peek() != Some(b'{') {
            let expr = self.parse_expr()?;
            return Ok(Pattern::Condition(expr));
        }
        Ok(Pattern::Always)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn is_word_boundary(&self, offset: usize) -> bool {
        let p = self.pos + offset;
        if p >= self.input.len() {
            return true;
        }
        let c = self.input.as_bytes()[p];
        !c.is_ascii_alphanumeric() && c != b'_'
    }

    fn parse_regex_literal(&mut self) -> Result<String, String> {
        self.expect(b'/')?;
        let start = self.pos;
        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if c == b'/' {
                let re = self.input[start..self.pos].to_string();
                self.advance();
                return Ok(re);
            }
            if c == b'\\' {
                self.advance(); // skip escaped char
            }
            self.advance();
        }
        Err("awk: unterminated regex".into())
    }

    fn parse_action_block(&mut self) -> Result<Vec<Action>, String> {
        self.expect(b'{')?;
        let mut actions = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.advance();
                break;
            }
            if self.pos >= self.input.len() {
                return Err("awk: unterminated action block".into());
            }
            // skip semicolons
            if self.peek() == Some(b';') {
                self.advance();
                continue;
            }
            actions.push(self.parse_action()?);
        }
        Ok(actions)
    }

    fn parse_action(&mut self) -> Result<Action, String> {
        self.skip_ws();
        if self.starts_with("print") && self.is_word_boundary(5) {
            self.pos += 5;
            return self.parse_print();
        }
        if self.starts_with("if") && self.is_word_boundary(2) {
            return Err("awk: not implemented: if statements".into());
        }
        if self.starts_with("for") && self.is_word_boundary(3) {
            return Err("awk: not implemented: for loops".into());
        }
        if self.starts_with("while") && self.is_word_boundary(5) {
            return Err("awk: not implemented: while loops".into());
        }
        if self.starts_with("function") && self.is_word_boundary(8) {
            return Err("awk: not implemented: user-defined functions".into());
        }
        if self.starts_with("printf") && self.is_word_boundary(6) {
            return Err("awk: not implemented: printf".into());
        }
        if self.starts_with("getline") && self.is_word_boundary(7) {
            return Err("awk: not implemented: getline".into());
        }
        if self.starts_with("next") && self.is_word_boundary(4) {
            return Err("awk: not implemented: next".into());
        }
        if self.starts_with("delete") && self.is_word_boundary(6) {
            return Err("awk: not implemented: delete".into());
        }
        // expression statement (e.g. sum += $1)
        let expr = self.parse_expr()?;
        Ok(Action::Expr(expr))
    }

    fn parse_print(&mut self) -> Result<Action, String> {
        self.skip_ws();
        if self.peek() == Some(b';') || self.peek() == Some(b'}') || self.pos >= self.input.len() {
            return Ok(Action::Print(vec![Expr::Field(Box::new(Expr::Literal(
                0.0,
            )))]));
        }
        let mut exprs = Vec::new();
        exprs.push(self.parse_expr()?);
        loop {
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.advance();
                self.skip_ws();
                exprs.push(self.parse_expr()?);
            } else {
                break;
            }
        }
        Ok(Action::Print(exprs))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let expr = self.parse_comparison()?;
        self.skip_ws();
        // check for assignment operators on variables
        if let Expr::Var(ref name) = expr {
            if self.peek() == Some(b'=') && self.peek2() != Some(b'=') {
                self.advance();
                let rhs = self.parse_expr()?;
                return Ok(Expr::Assign(name.clone(), Box::new(rhs)));
            }
            if self.starts_with("+=") {
                self.pos += 2;
                let rhs = self.parse_expr()?;
                return Ok(Expr::AssignOp(name.clone(), BinOp::Add, Box::new(rhs)));
            }
            if self.starts_with("-=") {
                self.pos += 2;
                let rhs = self.parse_expr()?;
                return Ok(Expr::AssignOp(name.clone(), BinOp::Sub, Box::new(rhs)));
            }
            if self.starts_with("*=") {
                self.pos += 2;
                let rhs = self.parse_expr()?;
                return Ok(Expr::AssignOp(name.clone(), BinOp::Mul, Box::new(rhs)));
            }
            if self.starts_with("/=") {
                self.pos += 2;
                let rhs = self.parse_expr()?;
                return Ok(Expr::AssignOp(name.clone(), BinOp::Div, Box::new(rhs)));
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_addition()?;
        loop {
            self.skip_ws();
            let op = if self.starts_with("==") {
                self.pos += 2;
                BinOp::Eq
            } else if self.starts_with("!=") {
                self.pos += 2;
                BinOp::Ne
            } else if self.starts_with(">=") {
                self.pos += 2;
                BinOp::Ge
            } else if self.starts_with("<=") {
                self.pos += 2;
                BinOp::Le
            } else if self.peek() == Some(b'>') {
                self.advance();
                BinOp::Gt
            } else if self.peek() == Some(b'<') {
                self.advance();
                BinOp::Lt
            } else if self.peek() == Some(b'~') {
                self.advance();
                self.skip_ws();
                let re = self.parse_regex_literal()?;
                left = Expr::Match(Box::new(left), re);
                continue;
            } else if self.starts_with("!~") {
                self.pos += 2;
                self.skip_ws();
                let re = self.parse_regex_literal()?;
                left = Expr::NotMatch(Box::new(left), re);
                continue;
            } else {
                break;
            };
            let right = self.parse_addition()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;
        loop {
            self.skip_ws();
            let op = if self.peek() == Some(b'+') && self.peek2() != Some(b'=') {
                self.advance();
                BinOp::Add
            } else if self.peek() == Some(b'-') && self.peek2() != Some(b'=') {
                self.advance();
                BinOp::Sub
            } else {
                break;
            };
            let right = self.parse_multiplication()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_concat()?;
        loop {
            self.skip_ws();
            let op = if self.peek() == Some(b'*') && self.peek2() != Some(b'=') {
                self.advance();
                BinOp::Mul
            } else if self.peek() == Some(b'/') && self.peek2() != Some(b'=') {
                self.advance();
                BinOp::Div
            } else if self.peek() == Some(b'%') {
                self.advance();
                BinOp::Mod
            } else {
                break;
            };
            let right = self.parse_concat()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<Expr, String> {
        let left = self.parse_primary()?;
        self.skip_ws();
        // string concatenation: two adjacent expressions with space between
        // only if next char could start an expression (not an operator or delimiter)
        if let Some(c) = self.peek() {
            if c == b'$' || c == b'"' || c.is_ascii_alphabetic() || c == b'(' {
                let right = self.parse_concat()?;
                return Ok(Expr::Concat(Box::new(left), Box::new(right)));
            }
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'$') => {
                self.advance();
                let idx = self.parse_primary()?;
                Ok(Expr::Field(Box::new(idx)))
            }
            Some(b'"') => {
                self.advance();
                let start = self.pos;
                while self.pos < self.input.len() {
                    if self.input.as_bytes()[self.pos] == b'"' {
                        let s = self.input[start..self.pos].to_string();
                        self.advance();
                        return Ok(Expr::StringLit(s));
                    }
                    if self.input.as_bytes()[self.pos] == b'\\' {
                        self.advance();
                    }
                    self.advance();
                }
                Err("awk: unterminated string".into())
            }
            Some(b'(') => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(b')')?;
                Ok(expr)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let c = self.input.as_bytes()[self.pos];
                    if c.is_ascii_digit() || c == b'.' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let num: f64 = self.input[start..self.pos]
                    .parse()
                    .map_err(|_| "awk: invalid number")?;
                Ok(Expr::Literal(num))
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self.pos < self.input.len() {
                    let c = self.input.as_bytes()[self.pos];
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let ident = &self.input[start..self.pos];
                match ident {
                    "NR" => Ok(Expr::NR),
                    "NF" => Ok(Expr::NF),
                    // built-in functions we don't support
                    "length" | "substr" | "index" | "split" | "sub" | "gsub" | "sprintf"
                    | "tolower" | "toupper" | "sin" | "cos" | "exp" | "log" | "sqrt" | "int" => {
                        Err(format!("awk: not implemented: {ident}()"))
                    }
                    _ => {
                        // check if it's an array access
                        self.skip_ws();
                        if self.peek() == Some(b'[') {
                            return Err("awk: not implemented: arrays".into());
                        }
                        Ok(Expr::Var(ident.to_string()))
                    }
                }
            }
            _ => Err(format!(
                "awk: unexpected character: '{}'",
                self.peek().map(|b| b as char).unwrap_or('\0')
            )),
        }
    }
}

struct Env {
    vars: std::collections::HashMap<String, f64>,
    ofs: String,
}

impl Env {
    fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
            ofs: " ".into(),
        }
    }

    fn eval(&mut self, expr: &Expr, fields: &[&str], nr: usize) -> Result<String, String> {
        match expr {
            Expr::Field(idx) => {
                let i = self.eval_num(idx, fields, nr)? as usize;
                if i == 0 {
                    Ok(fields.join(&self.ofs))
                } else {
                    Ok(fields.get(i - 1).unwrap_or(&"").to_string())
                }
            }
            Expr::Literal(n) => Ok(format_number(*n)),
            Expr::StringLit(s) => Ok(unescape(s)),
            Expr::NR => Ok(nr.to_string()),
            Expr::NF => Ok(fields.len().to_string()),
            Expr::Var(name) => {
                let v = self.vars.get(name).copied().unwrap_or(0.0);
                Ok(format_number(v))
            }
            Expr::BinOp(left, op, right) => {
                let l = self.eval_num(left, fields, nr)?;
                let r = self.eval_num(right, fields, nr)?;
                let result = match op {
                    BinOp::Add => l + r,
                    BinOp::Sub => l - r,
                    BinOp::Mul => l * r,
                    BinOp::Div => {
                        if r == 0.0 {
                            return Err("awk: division by zero".into());
                        }
                        l / r
                    }
                    BinOp::Mod => {
                        if r == 0.0 {
                            return Err("awk: division by zero".into());
                        }
                        l % r
                    }
                    BinOp::Gt => if l > r { 1.0 } else { 0.0 },
                    BinOp::Lt => if l < r { 1.0 } else { 0.0 },
                    BinOp::Ge => if l >= r { 1.0 } else { 0.0 },
                    BinOp::Le => if l <= r { 1.0 } else { 0.0 },
                    BinOp::Eq => if l == r { 1.0 } else { 0.0 },
                    BinOp::Ne => if l != r { 1.0 } else { 0.0 },
                };
                Ok(format_number(result))
            }
            Expr::Assign(name, val) => {
                let v = self.eval_num(val, fields, nr)?;
                self.vars.insert(name.clone(), v);
                Ok(format_number(v))
            }
            Expr::AssignOp(name, op, val) => {
                let current = self.vars.get(name).copied().unwrap_or(0.0);
                let rhs = self.eval_num(val, fields, nr)?;
                let result = match op {
                    BinOp::Add => current + rhs,
                    BinOp::Sub => current - rhs,
                    BinOp::Mul => current * rhs,
                    BinOp::Div => current / rhs,
                    _ => current,
                };
                self.vars.insert(name.clone(), result);
                Ok(format_number(result))
            }
            Expr::Concat(left, right) => {
                let l = self.eval(left, fields, nr)?;
                let r = self.eval(right, fields, nr)?;
                Ok(format!("{l}{r}"))
            }
            Expr::Match(expr, re) => {
                let s = self.eval(expr, fields, nr)?;
                let re = Regex::new(re).map_err(|e| format!("awk: invalid regex: {e}"))?;
                Ok(if re.is_match(&s) { "1" } else { "0" }.into())
            }
            Expr::NotMatch(expr, re) => {
                let s = self.eval(expr, fields, nr)?;
                let re = Regex::new(re).map_err(|e| format!("awk: invalid regex: {e}"))?;
                Ok(if re.is_match(&s) { "0" } else { "1" }.into())
            }
        }
    }

    fn eval_num(&mut self, expr: &Expr, fields: &[&str], nr: usize) -> Result<f64, String> {
        let s = self.eval(expr, fields, nr)?;
        Ok(s.parse::<f64>().unwrap_or(0.0))
    }
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn matches_pattern(
    pattern: &Pattern,
    line: &str,
    fields: &[&str],
    nr: usize,
    env: &mut Env,
) -> Result<bool, String> {
    match pattern {
        Pattern::Always => Ok(true),
        Pattern::Begin | Pattern::End => Ok(false),
        Pattern::Regex(re) => {
            let re = Regex::new(re).map_err(|e| format!("awk: invalid regex: {e}"))?;
            Ok(re.is_match(line))
        }
        Pattern::Condition(expr) => {
            let val = env.eval(expr, fields, nr)?;
            Ok(val != "0" && !val.is_empty())
        }
    }
}

pub fn run(args: &str, stdin: Option<String>) -> Result<String, String> {
    let opts = parse_opts(args)?;
    let mut parser = Parser::new(&opts.program);
    let rules = parser.parse_program()?;

    let input = if opts.paths.is_empty() {
        stdin.unwrap_or_default()
    } else {
        let mut combined = String::new();
        for path in &opts.paths {
            let content =
                fs::read_to_string(path).map_err(|e| format!("awk: {path}: {e}"))?;
            combined.push_str(&content);
        }
        combined
    };

    let mut env = Env::new();
    let mut output = String::new();

    // BEGIN rules
    for rule in &rules {
        if matches!(rule.pattern, Pattern::Begin) {
            for action in &rule.actions {
                exec_action(action, &[], 0, &mut env, &mut output)?;
            }
        }
    }

    // main rules
    for (nr, line) in input.lines().enumerate() {
        let nr = nr + 1;
        let fields: Vec<&str> = if opts.field_sep == " " {
            line.split_whitespace().collect()
        } else {
            line.split(&opts.field_sep).collect()
        };
        for rule in &rules {
            if matches_pattern(&rule.pattern, line, &fields, nr, &mut env)? {
                for action in &rule.actions {
                    exec_action(action, &fields, nr, &mut env, &mut output)?;
                }
            }
        }
    }

    // END rules
    for rule in &rules {
        if matches!(rule.pattern, Pattern::End) {
            for action in &rule.actions {
                exec_action(action, &[], 0, &mut env, &mut output)?;
            }
        }
    }

    Ok(output)
}

fn exec_action(
    action: &Action,
    fields: &[&str],
    nr: usize,
    env: &mut Env,
    output: &mut String,
) -> Result<(), String> {
    match action {
        Action::Print(exprs) => {
            let vals: Vec<String> = exprs
                .iter()
                .map(|e| env.eval(e, fields, nr))
                .collect::<Result<_, _>>()?;
            output.push_str(&vals.join(&env.ofs));
            output.push('\n');
        }
        Action::Expr(expr) => {
            env.eval(expr, fields, nr)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn awk(program: &str, input: &str) -> Result<String, String> {
        run(&format!("'{program}'"), Some(input.to_string()))
    }

    fn awk_f(sep: &str, program: &str, input: &str) -> Result<String, String> {
        run(&format!("-F{sep} '{program}'"), Some(input.to_string()))
    }

    #[test]
    fn print_first_field() {
        let out = awk("{print $1}", "hello world\nfoo bar").unwrap();
        assert_eq!(out, "hello\nfoo\n");
    }

    #[test]
    fn print_second_field() {
        let out = awk("{print $2}", "a b c\nd e f").unwrap();
        assert_eq!(out, "b\ne\n");
    }

    #[test]
    fn print_whole_line() {
        let out = awk("{print $0}", "hello world").unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn print_multiple_fields() {
        let out = awk("{print $1, $3}", "a b c d").unwrap();
        assert_eq!(out, "a c\n");
    }

    #[test]
    fn field_out_of_range() {
        let out = awk("{print $5}", "a b c").unwrap();
        assert_eq!(out, "\n");
    }

    #[test]
    fn colon_delimiter() {
        let out = awk_f(":", "{print $1}", "root:x:0:0").unwrap();
        assert_eq!(out, "root\n");
    }

    #[test]
    fn comma_delimiter() {
        let out = awk_f(",", "{print $2}", "a,b,c").unwrap();
        assert_eq!(out, "b\n");
    }

    #[test]
    fn regex_pattern() {
        let out = awk("/foo/ {print $0}", "foo bar\nbaz qux\nfooey").unwrap();
        assert_eq!(out, "foo bar\nfooey\n");
    }

    #[test]
    fn regex_pattern_no_action() {
        let out = awk("/hello/", "hello world\ngoodbye world").unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn condition_pattern() {
        let out = awk("NR == 2 {print $0}", "line1\nline2\nline3").unwrap();
        assert_eq!(out, "line2\n");
    }

    #[test]
    fn condition_gt() {
        let out = awk("$1 > 5 {print $1}", "3\n7\n1\n10").unwrap();
        assert_eq!(out, "7\n10\n");
    }

    #[test]
    fn begin_block() {
        let out = awk("BEGIN {print \"header\"} {print $1}", "a b\nc d").unwrap();
        assert_eq!(out, "header\na\nc\n");
    }

    #[test]
    fn end_block() {
        let out = awk("{print $1} END {print \"done\"}", "a\nb").unwrap();
        assert_eq!(out, "a\nb\ndone\n");
    }

    #[test]
    fn begin_end_only() {
        let out = awk("BEGIN {print \"start\"} END {print \"end\"}", "ignored").unwrap();
        assert_eq!(out, "start\nend\n");
    }

    #[test]
    fn addition() {
        let out = awk("{print $1 + $2}", "3 4\n10 20").unwrap();
        assert_eq!(out, "7\n30\n");
    }

    #[test]
    fn subtraction() {
        let out = awk("{print $1 - $2}", "10 3").unwrap();
        assert_eq!(out, "7\n");
    }

    #[test]
    fn multiplication() {
        let out = awk("{print $1 * $2}", "3 4").unwrap();
        assert_eq!(out, "12\n");
    }

    #[test]
    fn division() {
        let out = awk("{print $1 / $2}", "10 4").unwrap();
        assert_eq!(out, "2.5\n");
    }

    #[test]
    fn modulo() {
        let out = awk("{print $1 % $2}", "10 3").unwrap();
        assert_eq!(out, "1\n");
    }

    #[test]
    fn division_by_zero() {
        let out = awk("{print $1 / 0}", "5");
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("division by zero"));
    }

    #[test]
    fn sum_accumulator() {
        let out = awk("{sum += $1} END {print sum}", "10\n20\n30").unwrap();
        assert_eq!(out, "60\n");
    }

    #[test]
    fn assign_variable() {
        let out = awk("{x = $1 + 1; print x}", "5").unwrap();
        assert_eq!(out, "6\n");
    }

    #[test]
    fn count_lines() {
        let out = awk("{n += 1} END {print n}", "a\nb\nc\nd").unwrap();
        assert_eq!(out, "4\n");
    }

    #[test]
    fn nr() {
        let out = awk("{print NR, $0}", "a\nb\nc").unwrap();
        assert_eq!(out, "1 a\n2 b\n3 c\n");
    }

    #[test]
    fn nf() {
        let out = awk("{print NF}", "a b c\nd e\nf").unwrap();
        assert_eq!(out, "3\n2\n1\n");
    }

    #[test]
    fn string_literal() {
        let out = awk("{print \"hello\"}", "x").unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn string_concat() {
        let out = awk("{print $1 \"-\" $2}", "hello world").unwrap();
        assert_eq!(out, "hello-world\n");
    }

    #[test]
    fn escape_sequences() {
        let out = awk("{print \"a\\tb\"}", "x").unwrap();
        assert_eq!(out, "a\tb\n");
    }

    #[test]
    fn eq_comparison() {
        let out = awk("$1 == 2 {print $0}", "1\n2\n3").unwrap();
        assert_eq!(out, "2\n");
    }

    #[test]
    fn ne_comparison() {
        let out = awk("$1 != 2 {print $0}", "1\n2\n3").unwrap();
        assert_eq!(out, "1\n3\n");
    }

    #[test]
    fn le_comparison() {
        let out = awk("$1 <= 2 {print $0}", "1\n2\n3").unwrap();
        assert_eq!(out, "1\n2\n");
    }

    #[test]
    fn ge_comparison() {
        let out = awk("$1 >= 2 {print $0}", "1\n2\n3").unwrap();
        assert_eq!(out, "2\n3\n");
    }

    #[test]
    fn match_operator() {
        let out = awk("$1 ~ /^f/ {print $0}", "foo\nbar\nfiz").unwrap();
        assert_eq!(out, "foo\nfiz\n");
    }

    #[test]
    fn not_match_operator() {
        let out = awk("$1 !~ /^f/ {print $0}", "foo\nbar\nfiz").unwrap();
        assert_eq!(out, "bar\n");
    }

    #[test]
    fn multiple_rules() {
        let out = awk("/a/ {print \"found a\"} /b/ {print \"found b\"}", "a\nb\nab").unwrap();
        assert_eq!(out, "found a\nfound b\nfound a\nfound b\n");
    }

    #[test]
    fn print_bare() {
        let out = awk("{print}", "hello world").unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn if_not_implemented() {
        let out = awk("{if ($1 > 0) print}", "1");
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("not implemented: if"));
    }

    #[test]
    fn for_not_implemented() {
        let out = awk("{for (i=0;i<3;i++) print}", "1");
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("not implemented: for"));
    }

    #[test]
    fn arrays_not_implemented() {
        let out = awk("{a[1] = 1}", "x");
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("not implemented: arrays"));
    }

    #[test]
    fn function_not_implemented() {
        let out = awk("{print length($0)}", "hello");
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("not implemented: length()"));
    }

    #[test]
    fn empty_input() {
        let out = awk("{print $1}", "").unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_program() {
        let out = run("", Some("x".into()));
        assert!(out.is_err());
    }

    #[test]
    fn parenthesized_expr() {
        let out = awk("{print ($1 + $2) * $3}", "2 3 4").unwrap();
        assert_eq!(out, "20\n");
    }

    #[test]
    fn shell_split_basic() {
        let tokens = shell_split("'{print $1}' file.txt").unwrap();
        assert_eq!(tokens, vec!["{print $1}", "file.txt"]);
    }

    #[test]
    fn shell_split_double_quotes() {
        let tokens = shell_split("\"{print $1}\" file.txt").unwrap();
        assert_eq!(tokens, vec!["{print $1}", "file.txt"]);
    }

    #[test]
    fn shell_split_unterminated() {
        assert!(shell_split("'unterminated").is_err());
    }
}
