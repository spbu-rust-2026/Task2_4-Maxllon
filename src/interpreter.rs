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

fn parse(source: &str) -> Term {
    parse_term(source).unwrap_or(Term::Error)
}

pub(super) fn evaluate_source(source: &str) -> (String, String) {
    let term = parse(source);
    let display = term.to_string();
    let result = display_result(run(&term));
    (display, result)
}

fn parse_term(source: &str) -> Option<Term> {
    let source = source.trim();
    if source == "error" {
        return Some(Term::Error);
    }
    if let Some(body) = source.strip_prefix("(λ.") {
        if let Some(body) = body.strip_suffix(')') {
            return Some(Term::Fun(parse_term(body)?.into()));
        }
    }
    if let Some(body) = source.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        if let Some(separator) = application_separator(body) {
            return Some(Term::App(
                parse_term(&body[..separator])?.into(),
                parse_term(&body[separator..])?.into(),
            ));
        }
    }
    if let Some(index) = source.strip_prefix('i').and_then(|s| s.parse().ok()) {
        return Some(Term::Var(index));
    }
    None
}

fn application_separator(source: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut separator = None;
    for (index, character) in source.char_indices() {
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

#[cfg(test)]
#[path = "../tests/unit/interpreter.rs"]
mod tests;
