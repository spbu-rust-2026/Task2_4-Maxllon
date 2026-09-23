use super::longest_palindromic_substring;

fn assert_longest_palindrome(text: &str, expected_length: usize) {
    let palindrome = longest_palindromic_substring(text);

    assert!(
        text.contains(palindrome),
        "result must be a substring of input"
    );
    assert!(
        palindrome.chars().eq(palindrome.chars().rev()),
        "result must be a palindrome"
    );
    assert_eq!(palindrome.chars().count(), expected_length);
}

#[test]
fn first_example_from_statement() {
    assert_longest_palindrome("aabcbcbaa", 9);
}

#[test]
fn banana_example_from_statement() {
    assert_longest_palindrome("i love eating bananas", 5);
}

#[test]
fn even_length_palindrome() {
    assert_longest_palindrome("abba", 4);
}

#[test]
fn odd_length_palindrome() {
    assert_longest_palindrome("racecar", 7);
}

#[test]
fn no_palindrome_longer_than_one_character() {
    assert_longest_palindrome("abcdefg", 1);
}

#[test]
fn repeated_characters() {
    assert_longest_palindrome("aaaaaa", 6);
}

#[test]
fn empty_input() {
    assert_longest_palindrome("", 0);
}

#[test]
fn spaces_are_part_of_the_substring() {
    assert_longest_palindrome("a b a", 5);
}

#[test]
fn repeated_spaces_are_preserved_and_counted() {
    assert_longest_palindrome("a  a", 4);
}

#[test]
fn digits_can_form_a_palindrome() {
    assert_longest_palindrome("12321x", 5);
}

#[test]
fn any_of_two_equal_maxima_is_accepted() {
    assert_longest_palindrome("babad", 3);
}

#[test]
fn longest_palindrome_at_start() {
    assert_longest_palindrome("racecarxyz", 7);
}

#[test]
fn longest_palindrome_at_end() {
    assert_longest_palindrome("xyzracecar", 7);
}

#[test]
fn spaces_can_surround_a_palindrome() {
    assert_longest_palindrome(" abba ", 6);
}
