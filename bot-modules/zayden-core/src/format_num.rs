use core::fmt::NumBuffer;

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
    let mut buf = NumBuffer::<u64>::new();
    let digits = n.unsigned_abs().format_into(&mut buf);

    let len = digits.len();
    let commas = len.saturating_sub(1) / 3;
    let mut result = String::with_capacity(len + commas + usize::from(n < 0));

    if n < 0 {
        result.push('-');
    }

    for (i, c) in digits.chars().enumerate() {
        if i != 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }

    result
}
