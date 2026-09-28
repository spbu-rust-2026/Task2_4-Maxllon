mod interpreter;

#[cfg(test)]
mod palindrome;

fn main() {
    let source = std::env::args()
        .nth(1)
        .map(|path| std::fs::read_to_string(path).expect("could not read compiled term"))
        .unwrap_or_else(|| include_str!("i").to_owned());
    let (term, result) = interpreter::evaluate_source(&source);
    println!("Терм: {term}");
    println!("Результат: {result}");
}
