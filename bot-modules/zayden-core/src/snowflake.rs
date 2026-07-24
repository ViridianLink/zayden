#[must_use]
#[inline]
pub const fn as_i64(id: u64) -> i64 {
    id.cast_signed()
}

#[must_use]
#[inline]
pub const fn as_u64(n: i64) -> u64 {
    n.cast_unsigned()
}
