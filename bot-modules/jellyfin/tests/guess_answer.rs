//! Guess matching for `/jellyfin guess`. Typos are forgiven; sequel numbers are
//! not.

use jellyfin::games::answer::matches;

#[test]
fn an_exact_title_matches() {
    assert!(matches("WarGames: The Dead Code", "WarGames: The Dead Code"));
}

#[test]
fn punctuation_and_case_are_ignored() {
    assert!(matches("WarGames: The Dead Code", "war games the dead code"));
    assert!(matches("Spider-Man: No Way Home", "spider man no way home"));
}

#[test]
fn a_single_typo_in_a_long_title_is_forgiven() {
    assert!(matches("WarGames: The Dead Code", "WarGames: The Dead Cod"));
    assert!(matches("The Shawshank Redemption", "the shawshank redemtion"));
}

#[test]
fn a_leading_article_is_optional() {
    assert!(matches("The Matrix", "Matrix"));
    assert!(matches("Matrix", "The Matrix"));
}

#[test]
fn an_ampersand_matches_the_word() {
    assert!(matches("Fast & Furious", "Fast and Furious"));
}

#[test]
fn accents_are_folded() {
    assert!(matches("Amélie", "amelie"));
    assert!(matches("Léon: The Professional", "leon the professional"));
}

#[test]
fn a_trailing_year_is_ignored() {
    assert!(matches("The Thing", "The Thing (1982)"));
}

#[test]
fn sequel_numbers_must_match() {
    assert!(!matches("Fast and Furious 5", "Fast and Furious 6"));
    assert!(!matches("Fast and Furious 5", "Fast and Furious"));
    assert!(!matches("Fast and Furious", "Fast and Furious 5"));
}

#[test]
fn roman_numerals_are_read_as_numbers() {
    assert!(matches("Rocky II", "Rocky 2"));
    assert!(!matches("Rocky II", "Rocky III"));
    assert!(!matches("Rocky II", "Rocky 3"));
}

#[test]
fn a_year_shaped_title_is_still_a_number() {
    assert!(matches("1917", "1917"));
    assert!(!matches("1917", "2012"));
}

#[test]
fn short_titles_must_be_exact() {
    assert!(!matches("Alien", "Aliens"));
    assert!(!matches("Up", "Us"));
}

#[test]
fn a_different_film_does_not_match() {
    assert!(!matches("WarGames: The Dead Code", "WarGames"));
    assert!(!matches("The Shawshank Redemption", "The Green Mile"));
    assert!(!matches("The Matrix", ""));
}
