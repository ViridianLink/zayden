use std::ops::{Deref, DerefMut};
use std::ptr;

const MAX_BUF_LEN: usize = 39 + 18 * MAX_SEP_LEN + MAX_MIN_LEN;
const MAX_MIN_LEN: usize = 8;
const MAX_SEP_LEN: usize = 8;

const ONE_TO_NINTY_NINE: &[u8] = b"\
    0001020304050607080910111213141516171819\
    2021222324252627282930313233343536373839\
    4041424344454647484950515253545556575859\
    6061626364656667686970717273747576777879\
    8081828384858687888990919293949596979899";

const MINUS_SIGN: &str = "-";
const MIN_LEN: usize = MINUS_SIGN.len();

const SEPARATOR: &str = ",";

struct Buffer {
    inner: [u8; MAX_BUF_LEN],
    pos: usize,
    end: usize,
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer {
            inner: [0; MAX_BUF_LEN],
            pos: MAX_BUF_LEN,
            end: MAX_BUF_LEN,
        }
    }
}

impl Deref for Buffer {
    type Target = [u8; MAX_BUF_LEN];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub trait FormatNum {
    fn format(&self) -> String;
}

impl FormatNum for i64 {
    fn format(&self) -> String {
        let mut buf = Buffer::new();

        if self.is_negative() {
            let n = (!(*self as u128)).wrapping_add(1); // make positive by adding 1 to the 2s complement
            run_core_algorithm(n, &mut buf);

            buf.pos -= MIN_LEN;
            for (i, byte) in MINUS_SIGN.as_bytes().iter().enumerate() {
                buf.inner[buf.pos + i] = *byte;
            }
        } else {
            let n = *self as u128;
            run_core_algorithm(n, &mut buf);
        }

        std::str::from_utf8(&buf[buf.pos..buf.end])
            .unwrap()
            .to_string()
    }
}

impl FormatNum for i32 {
    fn format(&self) -> String {
        let mut buf = Buffer::new();

        if self.is_negative() {
            let n = (!(*self as u128)).wrapping_add(1); // make positive by adding 1 to the 2s complement
            run_core_algorithm(n, &mut buf);

            buf.pos -= MIN_LEN;
            for (i, byte) in MINUS_SIGN.as_bytes().iter().enumerate() {
                buf.inner[buf.pos + i] = *byte;
            }
        } else {
            let n = *self as u128;
            run_core_algorithm(n, &mut buf);
        }

        std::str::from_utf8(&buf[buf.pos..buf.end])
            .unwrap()
            .to_string()
    }
}

fn run_core_algorithm(mut n: u128, buf: &mut Buffer) -> usize {
    // Reset position to the end of the buffer
    buf.pos = MAX_BUF_LEN;
    buf.end = MAX_BUF_LEN;

    let mut sep = Sep::new();

    while n >= 10_000 {
        let remainder = n % 10_000;
        let table_index = ((remainder % 100) << 1) as isize;
        write_two_bytes(buf, &mut sep, table_index);
        let table_index = ((remainder / 100) << 1) as isize;
        write_two_bytes(buf, &mut sep, table_index);
        n /= 10_000;
    }
    let mut n = n as isize;
    while n >= 100 {
        let table_index = (n % 100) << 1;
        write_two_bytes(buf, &mut sep, table_index);
        n /= 100;
    }
    if n >= 10 {
        let table_index = n << 1;
        write_two_bytes(buf, &mut sep, table_index);
    } else {
        let table_index = n << 1;
        write_one_byte(buf, &mut sep, table_index + 1);
    }

    buf.end - buf.pos
}

struct Sep {
    ptr: *const u8,
    len: usize,
    pos: isize,
    step: isize,
}

impl Sep {
    fn new() -> Self {
        Self {
            ptr: SEPARATOR.as_bytes().as_ptr(),
            len: SEPARATOR.len(),
            pos: MAX_BUF_LEN as isize - 4,
            step: 4isize,
        }
    }
}

fn write_one_byte(buf: &mut Buffer, sep: &mut Sep, table_index: isize) {
    buf.pos -= 1;
    if sep.pos == (buf.pos as isize) {
        buf.pos -= sep.len - 1;
        unsafe { ptr::copy_nonoverlapping(sep.ptr, buf.as_mut_ptr().add(buf.pos), sep.len) }
        sep.pos -= sep.step + (sep.len as isize - 1);
        buf.pos -= 1;
    }
    unsafe {
        ptr::copy_nonoverlapping(
            ONE_TO_NINTY_NINE.as_ptr().offset(table_index),
            buf.as_mut_ptr().add(buf.pos),
            1,
        )
    };
}

fn write_two_bytes(buf: &mut Buffer, sep: &mut Sep, table_index: isize) {
    write_one_byte(buf, sep, table_index + 1);
    write_one_byte(buf, sep, table_index);
}
