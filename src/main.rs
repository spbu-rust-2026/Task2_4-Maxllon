mod interpreter;

#[cfg(test)]
mod palindrome;

fn main() {
    let input = vec![1, 7, 4, 2, 3];
    let mut show_term = false;
    let mut source_path = None;
    for argument in std::env::args().skip(1) {
        if argument == "--show-term" {
            show_term = true;
        } else {
            source_path = Some(argument);
        }
    }

    let source = source_path
        .map(|path| std::fs::read_to_string(path).expect("could not read compiled term"))
        .unwrap_or_else(|| include_str!("i").to_owned());
    let (term, result) = interpreter::evaluate_source_with_list(&source, show_term, &input);
    if let Some(term) = term {
        println!("Терм: {term}");
    }
    println!("Результат: {result}");
}
