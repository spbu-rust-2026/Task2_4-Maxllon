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
    let mut parser = Parser {
        source: source.trim(),
        position: 0,
    };
    let term = parser.parse_term();
    if parser.position == parser.source.len() {
        term.unwrap_or(Term::Error)
    } else {
        Term::Error
    }
}

pub(super) fn evaluate_source_with_list(
    source: &str,
    show_term: bool,
    numbers: &[i32],
) -> (Option<String>, String) {
    let term = parse(source);
    let input = church_list(numbers);
    let display = show_term.then(|| term.to_string());
    let mut context = EvalContext::new();
    let function = context.eval(0, &term);
    let list = context.eval(0, &input);
    let value = context.apply(function, list);
    let result = display_result(&mut context, value);
    (display, result)
}

fn church_list(numbers: &[i32]) -> Term {
    let mut list = Term::App(
        constructor(0, 2).into(),
        Term::Fun(Term::Var(0).into()).into(),
    );
    for number in numbers.iter().rev() {
        let tail = Term::Fun(
            Term::App(
                Term::App(Term::Var(0).into(), list.into()).into(),
                Term::Fun(Term::Var(0).into()).into(),
            )
            .into(),
        );
        let tuple = Term::Fun(
            Term::App(
                Term::App(Term::Var(0).into(), church_integer(*number).into()).into(),
                tail.into(),
            )
            .into(),
        );
        list = Term::App(constructor(1, 2).into(), tuple.into());
    }
    list
}

fn constructor(index: usize, total: usize) -> Term {
    let mut term = Term::App(Term::Var(total - index - 1).into(), Term::Var(total).into());
    for _ in 0..total {
        term = Term::Fun(term.into());
    }
    Term::Fun(term.into())
}

fn church_integer(number: i32) -> Term {
    let mut term = Term::Fun(Term::Var(0).into());
    for bit in (0..32).rev() {
        let boolean = Term::Fun(
            Term::Fun(Term::Var(if number & (1 << bit) != 0 { 1 } else { 0 }).into()).into(),
        );
        term = Term::Fun(
            Term::App(
                Term::App(Term::Var(0).into(), boolean.into()).into(),
                term.into(),
            )
            .into(),
        );
    }
    term
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
}

impl Parser<'_> {
    fn parse_term(&mut self) -> Option<Term> {
        let bytes = self.source.as_bytes();
        match *bytes.get(self.position)? {
            b'(' => {
                self.position += 1;
                if self.source[self.position..].starts_with("λ.") {
                    self.position += "λ.".len();
                    let body = self.parse_term()?;
                    self.expect(b')')?;
                    Some(Term::Fun(body.into()))
                } else {
                    let function = self.parse_term()?;
                    self.expect(b' ')?;
                    let argument = self.parse_term()?;
                    self.expect(b')')?;
                    Some(Term::App(function.into(), argument.into()))
                }
            }
            b'i' => {
                self.position += 1;
                let start = self.position;
                while bytes.get(self.position).is_some_and(u8::is_ascii_digit) {
                    self.position += 1;
                }
                (self.position > start)
                    .then(|| self.source[start..self.position].parse().ok())
                    .flatten()
                    .map(Term::Var)
            }
            b'e' if self.source[self.position..].starts_with("error") => {
                self.position += "error".len();
                Some(Term::Error)
            }
            _ => None,
        }
    }

    fn expect(&mut self, expected: u8) -> Option<()> {
        if self.source.as_bytes().get(self.position) == Some(&expected) {
            self.position += 1;
            Some(())
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Value<'a> {
    #[allow(dead_code)]
    Int(i32),
    Bool(bool),
    Closure(&'a Term, usize),
    Error,
}

struct EnvEntry<'a> {
    value: Option<Value<'a>>,
    parent: usize,
}

struct EvalContext<'a> {
    // Index zero is the empty environment; all other nodes are immutable after
    // insertion, so closures can retain an index while the arena grows.
    environments: Vec<EnvEntry<'a>>,
}

impl<'a> EvalContext<'a> {
    fn new() -> Self {
        Self {
            environments: vec![EnvEntry {
                value: None,
                parent: 0,
            }],
        }
    }

    fn extend(&mut self, parent: usize, value: Value<'a>) -> usize {
        let index = self.environments.len();
        self.environments.push(EnvEntry {
            value: Some(value),
            parent,
        });
        index
    }

    fn get(&self, mut environment: usize, mut index: usize) -> Value<'a> {
        loop {
            let entry = self
                .environments
                .get(environment)
                .filter(|_| environment != 0)
                .expect("unbound de Bruijn index");
            if index == 0 {
                return entry.value.expect("non-empty environment node");
            }
            index -= 1;
            environment = entry.parent;
        }
    }

    fn apply(&mut self, function: Value<'a>, argument: Value<'a>) -> Value<'a> {
        match function {
            Value::Closure(body, environment) => {
                let environment = self.extend(environment, argument);
                self.eval(environment, body)
            }
            Value::Error | Value::Int(_) | Value::Bool(_) => Value::Error,
        }
    }

    fn eval(&mut self, environment: usize, term: &'a Term) -> Value<'a> {
        match term {
            Term::Var(index) => self.get(environment, *index),
            Term::Fun(body) => Value::Closure(body, environment),
            Term::App(function, argument) => {
                let function_value = self.eval(environment, function);
                if matches!(function_value, Value::Error) {
                    return Value::Error;
                }

                let argument_value = self.eval(environment, argument);
                if matches!(argument_value, Value::Error) {
                    return Value::Error;
                }

                self.apply(function_value, argument_value)
            }
            Term::Error => Value::Error,
        }
    }
}

fn as_church_bool<'a>(context: &mut EvalContext<'a>, value: Value<'a>) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(value),
        Value::Closure(Term::Fun(body), _) => match body.as_ref() {
            Term::Var(0) => Some(false),
            Term::Var(1) => Some(true),
            _ => apply_church_bool(context, value),
        },
        Value::Closure(Term::App(function, argument), environment) => {
            let function = context.eval(environment, function);
            let argument = context.eval(environment, argument);
            let reduced = context.apply(function, argument);
            as_church_bool(context, reduced)
        }
        Value::Closure(_, _) => apply_church_bool(context, value),
        Value::Int(_) | Value::Error => None,
    }
}

fn apply_church_bool<'a>(context: &mut EvalContext<'a>, value: Value<'a>) -> Option<bool> {
    let first = context.apply(value, Value::Bool(true));
    match context.apply(first, Value::Bool(false)) {
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

fn church_tuple_selectors() -> &'static [Term] {
    static SELECTORS: std::sync::OnceLock<Vec<Term>> = std::sync::OnceLock::new();
    SELECTORS.get_or_init(|| (0..32).map(church_tuple_selector).collect())
}

fn as_church_int<'a>(context: &mut EvalContext<'a>, value: Value<'a>) -> Option<i32> {
    let Value::Closure(body, _) = value else {
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

    for (index, selector) in church_tuple_selectors().iter().enumerate() {
        let selected = context.apply(Value::Closure(selector, 0), value);
        if as_church_bool(context, selected)? {
            bits |= 1 << index;
        }
    }
    Some(bits as i32)
}

fn display_result<'a>(value_context: &mut EvalContext<'a>, value: Value<'a>) -> String {
    match value {
        Value::Int(number) => number.to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Closure(Term::Fun(body), _) if matches!(body.as_ref(), Term::Var(0 | 1)) => {
            as_church_bool(value_context, value).unwrap().to_string()
        }
        Value::Closure(_, _) => match as_church_int(value_context, value) {
            Some(number) => number.to_string(),
            None => "function".to_owned(),
        },
        Value::Error => "error".to_owned(),
    }
}
