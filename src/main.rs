#[derive(Debug, Clone)]
enum Term {
    Var(usize),
    Fun(Box<Term>),
    App(Box<Term>, Box<Term>),
    Error,
}
fn parse(s: &str) -> Term {
    let s = s.trim();
    if let Some(b) = s.strip_prefix("(λ.") { return Term::Fun(parse(&b[..b.len() - 1]).into()) }
    if let Some(i) = s.char_indices().rev().scan(0i32, |d, (i, c)| { *d += (c == ')') as i32 - (c == '(') as i32; Some((i, *d, c)) }).find(|&(_, d, c)| c == ' ' && d == 1).map(|(i, _, _)| i) { return Term::App(parse(&s[1..i]).into(), parse(&s[i + 1..s.len() - 1]).into()) }
    if let Some(i) = s.strip_prefix('i') { return Term::Var(i.parse().unwrap()) }
    Term::Error
}
fn main() {
    let _ = parse(include_str!("i"));
}
#[cfg(test)]
#[path = "../tests/unit/palindrome.rs"]
mod palindrome_tests;
