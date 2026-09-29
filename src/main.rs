mod interpreter;
use std::io::{self, Read};

fn main() {
    let mut text = String::new();
    io::stdin()
        .read_to_string(&mut text)
        .expect("failed to read input");

    let text = text.trim_end_matches(|character| character == '\n' || character == '\r');
    println!("{}", longest_palindromic_substring(text));
}

fn longest_palindromic_substring(_text: &str) -> &str {
    let input: Vec<i32> = _text.as_bytes().iter().map(|&b| b as i32).collect();

    let source = include_str!("hello_world.py");
    let (_, result) = interpreter::evaluate_source_with_list(&source, false, &input);
    let res: usize = match result.parse() {
        Ok(v) => v,
        Err(_) => return "",
    };
    let res1 = res / (1024 * 64);
    let res2 = res % (1024 * 64);
    &_text[res2..res1 + res2]
}

#[cfg(test)]
#[path = "../tests/unit/palindrome.rs"]
mod palindrome_tests;
