pub trait FormatNum {
    #[must_use]
    fn format(&self) -> String;
}

impl FormatNum for i64 {
    fn format(&self) -> String {
        format_with_commas(*self)
    }
}

impl FormatNum for i32 {
    fn format(&self) -> String {
        format_with_commas(i64::from(*self))
    }
}

fn format_with_commas(n: i64) -> String {
    let is_negative = n < 0;
    let abs = n.unsigned_abs();
    let s = abs.to_string();
    let len = s.len();
    let commas = len.saturating_sub(1) / 3;
    let mut result = String::with_capacity(len + commas + usize::from(is_negative));

    for (i, c) in s.chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    if is_negative {
        result.push('-');
    }

    result.chars().rev().collect()
}
