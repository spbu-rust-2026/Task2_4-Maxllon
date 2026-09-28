use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Term {
    Var(usize),
    Fun(Box<Term>),
    App(Box<Term>, Box<Term>),
    Error,
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Var(index) => write!(f, "i{index}"),
            Term::Fun(body) => write!(f, "(λ.{body})"),
            Term::App(function, argument) => write!(f, "({function} {argument})"),
            Term::Error => write!(f, "error"),
        }
    }
}

fn parse(s: &str) -> Term {
    parse_term(s).unwrap_or(Term::Error)
}

fn parse_term(s: &str) -> Option<Term> {
    let s = s.trim();
    if s == "error" {
        return Some(Term::Error);
    }
    if let Some(body) = s.strip_prefix("(λ.") {
        if let Some(body) = body.strip_suffix(')') {
            return Some(Term::Fun(parse_term(body)?.into()));
        }
    }
    if let Some(body) = s.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        if let Some(i) = application_separator(body) {
            return Some(Term::App(
                parse_term(&body[..i])?.into(),
                parse_term(&body[i..])?.into(),
            ));
        }
    }
    if let Some(i) = s.strip_prefix('i') {
        if let Ok(index) = i.parse() {
            return Some(Term::Var(index));
        }
    }
    None
}

fn application_separator(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut separator = None;
    for (index, character) in s.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ' ' if depth == 0 => separator = Some(index),
            _ => {}
        }
    }
    separator
}

#[derive(Clone, Debug, PartialEq)]
struct Env<'a>(Option<Rc<EnvEntry<'a>>>);

#[derive(Clone, Debug, PartialEq)]
struct EnvEntry<'a> {
    value: Value<'a>,
    parent: Env<'a>,
}

impl<'a> Env<'a> {
    fn empty() -> Self {
        Self(None)
    }

    fn extend(&self, value: Value<'a>) -> Self {
        Self(Some(Rc::new(EnvEntry {
            value,
            parent: self.clone(),
        })))
    }

    fn get(&self, mut index: usize) -> &Value<'a> {
        let mut current = self;
        loop {
            let entry = current.0.as_deref().expect("unbound de Bruijn index");
            if index == 0 {
                return &entry.value;
            }
            index -= 1;
            current = &entry.parent;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Value<'a> {
    #[allow(dead_code)]
    Int(i32),
    Bool(bool),
    Closure(&'a Term, Env<'a>),
    Error,
}

impl std::fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(value) => write!(f, "{value}"),
            Value::Bool(value) => write!(f, "{}", if *value { "true" } else { "false" }),
            Value::Closure(_, _) => write!(f, "function"),
            Value::Error => write!(f, "error"),
        }
    }
}

fn apply_value<'a>(function: Value<'a>, argument: Value<'a>) -> Value<'a> {
    match function {
        Value::Closure(body, env) => eval(&env.extend(argument), body),
        Value::Error | Value::Int(_) | Value::Bool(_) => Value::Error,
    }
}

fn as_church_bool<'a>(value: Value<'a>) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(value),
        Value::Closure(Term::Fun(body), _) => match body.as_ref() {
            Term::Var(0) => Some(false),
            Term::Var(1) => Some(true),
            _ => apply_church_bool(value),
        },
        Value::Closure(Term::App(function, argument), env) => {
            let reduced = apply_value(eval(&env, function), eval(&env, argument));
            as_church_bool(reduced)
        }
        Value::Closure(_, _) => apply_church_bool(value),
        Value::Int(_) | Value::Error => None,
    }
}

fn apply_church_bool<'a>(value: Value<'a>) -> Option<bool> {
    let first = apply_value(value, Value::Bool(true));
    match apply_value(first, Value::Bool(false)) {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}

fn church_true() -> Term {
    Term::Fun(Term::Fun(Term::Var(1).into()).into())
}

fn church_false() -> Term {
    Term::Fun(Term::Fun(Term::Var(0).into()).into())
}

fn church_tuple_selector(index: usize) -> Term {
    let first = Term::Fun(Term::App(Term::Var(0).into(), church_true().into()).into());
    let second = Term::Fun(Term::App(Term::Var(0).into(), church_false().into()).into());
    let mut selection = Term::Var(0);
    for _ in 0..index {
        selection = Term::App(second.clone().into(), selection.into());
    }
    Term::Fun(Term::App(first.into(), selection.into()).into())
}

fn as_church_int<'a>(value: Value<'a>) -> Option<i32> {
    let Value::Closure(body, _) = &value else {
        return None;
    };

    fn bool_bit(term: &Term) -> Option<bool> {
        let Term::Fun(first) = term else {
            return None;
        };
        let Term::Fun(second) = first.as_ref() else {
            return None;
        };
        match second.as_ref() {
            Term::Var(0) => Some(false),
            Term::Var(1) => Some(true),
            _ => None,
        }
    }

    fn bits_from_tuple(term: &Term, index: u32, bits: &mut u32) -> Option<()> {
        if index == 32 {
            return matches!(term, Term::Var(0)).then_some(());
        }
        let Term::App(pair, rest) = term else {
            return None;
        };
        let Term::App(function, bit) = pair.as_ref() else {
            return None;
        };
        if !matches!(function.as_ref(), Term::Var(0)) {
            return None;
        }
        if bool_bit(bit)? {
            *bits |= 1 << index;
        }
        let Term::Fun(rest_body) = rest.as_ref() else {
            return None;
        };
        bits_from_tuple(rest_body, index + 1, bits)
    }

    let mut bits = 0u32;
    let direct_tuple = match body {
        Term::App(pair, _) => match pair.as_ref() {
            Term::App(function, _) if matches!(function.as_ref(), Term::Var(0)) => {
                bits_from_tuple(body, 0, &mut bits).is_some()
            }
            _ => false,
        },
        _ => false,
    };
    if direct_tuple {
        return Some(bits as i32);
    }

    for index in 0..32 {
        let selector = church_tuple_selector(index);
        let selected = apply_value(Value::Closure(&selector, Env::empty()), value.clone());
        if as_church_bool(selected)? {
            bits |= 1 << index;
        }
    }
    Some(bits as i32)
}

fn eval<'a>(env: &Env<'a>, term: &'a Term) -> Value<'a> {
    match term {
        Term::Var(index) => env.get(*index).clone(),
        Term::Fun(body) => Value::Closure(body, env.clone()),
        Term::App(function, argument) => {
            let function_value = eval(env, function);
            if matches!(function_value, Value::Error) {
                return Value::Error;
            }

            let argument_value = eval(env, argument);
            if matches!(argument_value, Value::Error) {
                return Value::Error;
            }

            match function_value {
                closure @ Value::Closure(_, _) => apply_value(closure, argument_value),
                Value::Int(_) | Value::Bool(_) => Value::Error,
                Value::Error => Value::Error,
            }
        }
        Term::Error => Value::Error,
    }
}

fn run(term: &Term) -> Value<'_> {
    eval(&Env::empty(), term)
}

fn display_result(value: Value<'_>) -> String {
    match value {
        Value::Int(number) => number.to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Closure(Term::Fun(body), _) if matches!(body.as_ref(), Term::Var(0 | 1)) => {
            as_church_bool(value).unwrap().to_string()
        }
        Value::Closure(_, _) => match as_church_int(value) {
            Some(number) => number.to_string(),
            None => "function".to_owned(),
        },
        Value::Error => "error".to_owned(),
    }
}

fn main() {
    let source = std::env::args()
        .nth(1)
        .map(|path| std::fs::read_to_string(path).expect("could not read compiled term"))
        .unwrap_or_else(|| include_str!("i").to_owned());
    let term = parse(&source);
    println!("Терм: {term}");
    println!("Результат: {}", display_result(run(&term)));
}

#[cfg(test)]
#[path = "../tests/unit/palindrome.rs"]
mod palindrome_tests;

#[cfg(test)]
fn longest_palindromic_substring(text: &str) -> &str {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return text;
    }

    let mut offsets: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();
    offsets.push(text.len());
    let mut best_start = 0;
    let mut best_length = 1;

    for center in 0..chars.len() {
        let mut radius = 0;
        while radius <= center
            && center + radius < chars.len()
            && chars[center - radius] == chars[center + radius]
        {
            let length = radius * 2 + 1;
            if length > best_length {
                best_start = center - radius;
                best_length = length;
            }
            radius += 1;
        }

        let mut radius = 0;
        while radius < center
            && center + radius < chars.len()
            && chars[center - radius - 1] == chars[center + radius]
        {
            let length = radius * 2 + 2;
            if length > best_length {
                best_start = center - radius - 1;
                best_length = length;
            }
            radius += 1;
        }
    }

    &text[offsets[best_start]..offsets[best_start + best_length]]
}

#[cfg(test)]
mod term_tests {
    use super::{
        Env, Term, Value, apply_value, as_church_bool, as_church_int, church_tuple_selector,
        display_result, eval, parse, run,
    };

    fn church_boolean(value: bool) -> Term {
        Term::Fun(Term::Fun(Term::Var(if value { 1 } else { 0 }).into()).into())
    }

    fn church_integer(value: i32) -> Term {
        let mut tuple = Term::Fun(Term::Var(0).into());
        for bit in (0..32).rev() {
            tuple = Term::Fun(
                Term::App(
                    Term::App(
                        Term::Var(0).into(),
                        church_boolean(value & (1 << bit) != 0).into(),
                    )
                    .into(),
                    tuple.into(),
                )
                .into(),
            );
        }
        tuple
    }

    #[test]
    fn parses_and_displays_pure_lambda_terms() {
        let term = parse("((λ.(i1 i0)) i2)");

        assert_eq!(term.to_string(), "((λ.(i1 i0)) i2)");
    }

    #[test]
    fn unsupported_terms_are_errors() {
        assert!(matches!(parse("123"), Term::Error));
        assert!(matches!(parse("name"), Term::Error));
        assert!(matches!(parse("(try error with i0)"), Term::Error));
    }

    #[test]
    fn eval_applies_closures_using_the_captured_environment() {
        let term = parse("(((λ.(λ.i1)) i1) i0)");
        let env = Env::empty()
            .extend(Value::Int(42))
            .extend(Value::Bool(true));

        assert_eq!(eval(&env, &term), Value::Int(42));
    }

    #[test]
    fn eval_propagates_errors_and_skips_an_error_function_argument() {
        let term = Term::App(Term::Error.into(), Term::Var(99).into());

        assert!(matches!(run(&term), Value::Error));
    }

    #[test]
    fn eval_returns_the_argument_for_identity_application() {
        let term = parse("((λ.i0) (λ.i0))");
        let env = Env::empty();

        assert!(matches!(eval(&env, &term), Value::Closure(_, _)));
    }

    #[test]
    fn variables_return_values_from_the_environment() {
        let term = Term::Var(1);
        let env = Env::empty()
            .extend(Value::Int(7))
            .extend(Value::Bool(false));

        assert_eq!(eval(&env, &term), Value::Int(7));
    }

    #[test]
    fn applying_a_non_closure_returns_an_error() {
        let term = Term::App(Term::Var(0).into(), Term::Var(1).into());
        let env = Env::empty()
            .extend(Value::Bool(true))
            .extend(Value::Int(12));

        assert_eq!(eval(&env, &term), Value::Error);
    }

    #[test]
    fn decodes_prelude_integer_representation() {
        for expected in [0, 1, 42, -17] {
            let term = church_integer(expected);
            let value = run(&term);

            assert_eq!(as_church_int(value.clone()), Some(expected));
            assert_eq!(display_result(value), expected.to_string());
        }
    }

    #[test]
    fn decodes_integer_bits_after_beta_reduction() {
        let integer = church_integer(42);
        for (index, expected) in [(0, false), (1, true), (3, true)] {
            let selector = church_tuple_selector(index);
            let selected = apply_value(Value::Closure(&selector, Env::empty()), run(&integer));
            assert_eq!(as_church_bool(selected), Some(expected), "bit {index}");
        }
    }

    #[test]
    fn church_boolean_is_not_misidentified_as_an_integer() {
        assert_eq!(display_result(run(&church_boolean(true))), "true");
        assert_eq!(display_result(run(&church_boolean(false))), "false");
    }
}
