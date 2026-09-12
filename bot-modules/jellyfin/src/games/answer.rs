const ROMAN: &[(&str, u32)] = &[
    ("ii", 2),
    ("iii", 3),
    ("iv", 4),
    ("vi", 6),
    ("vii", 7),
    ("viii", 8),
    ("ix", 9),
    ("xi", 11),
    ("xii", 12),
    ("xiii", 13),
];

const ARTICLES: &[&str] = &["the", "a", "an"];

const LETTERS_PER_TYPO: usize = 8;

#[must_use]
pub fn matches(answer: &str, guess: &str) -> bool {
    let answer = normalize(answer);
    let guess = normalize(guess);

    if answer.numbers != guess.numbers {
        return false;
    }

    if answer.letters == guess.letters {
        return true;
    }

    let tolerance = answer.letters.len() / LETTERS_PER_TYPO;
    let length_gap = answer.letters.len().abs_diff(guess.letters.len());

    if tolerance == 0 || length_gap > tolerance {
        return false;
    }

    distance(&answer.letters, &guess.letters) <= tolerance
}

#[derive(Debug, PartialEq, Eq)]
struct Normalized {
    letters: Vec<char>,
    numbers: Vec<u32>,
}

fn normalize(raw: &str) -> Normalized {
    let folded = fold(strip_year(raw.trim()));

    let mut letters = Vec::with_capacity(folded.len());
    let mut numbers = Vec::new();
    let mut leading = true;

    for token in folded.split(|c: char| !c.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }

        if let Ok(value) = token.parse::<u32>() {
            numbers.push(value);
        } else if let Some((_, value)) =
            ROMAN.iter().find(|(numeral, _)| *numeral == token)
        {
            numbers.push(*value);
        } else if !(leading && ARTICLES.contains(&token)) {
            letters.extend(token.chars());
        }

        leading = false;
    }

    Normalized { letters, numbers }
}

fn strip_year(raw: &str) -> &str {
    let Some(head) = raw.strip_suffix(')') else {
        return raw;
    };

    let Some(open) = head.rfind('(') else {
        return raw;
    };

    let Some(year) = head.get(open + 1..) else {
        return raw;
    };

    if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
        head.get(..open).unwrap_or(raw).trim_end()
    } else {
        raw
    }
}

fn fold(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());

    for c in raw.chars().flat_map(char::to_lowercase) {
        match c {
            'à'..='å' => out.push('a'),
            'ç' => out.push('c'),
            'è'..='ë' => out.push('e'),
            'ì'..='ï' => out.push('i'),
            'ñ' => out.push('n'),
            'ò'..='ö' | 'ø' => out.push('o'),
            'ù'..='ü' => out.push('u'),
            'ý' | 'ÿ' => out.push('y'),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            'ß' => out.push_str("ss"),
            '&' => out.push_str(" and "),
            _ => out.push(c),
        }
    }

    out
}

fn distance(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = Vec::with_capacity(b.len() + 1);

    for (row, left_char) in a.iter().enumerate() {
        curr.clear();

        let mut left = row + 1;
        curr.push(left);

        for (column, right_char) in b.iter().enumerate() {
            let diagonal = prev.get(column).copied().unwrap_or(0);
            let above = prev.get(column + 1).copied().unwrap_or(0);
            let substitution = diagonal + usize::from(left_char != right_char);

            left = (above + 1).min(left + 1).min(substitution);
            curr.push(left);
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    prev.last().copied().unwrap_or(0)
}
