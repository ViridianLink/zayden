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

#[cfg(test)]
mod tests {
    use super::{as_i64, as_u64};

    #[test]
    fn round_trip_zero() {
        assert_eq!(as_u64(as_i64(0)), 0u64);
    }

    #[test]
    fn round_trip_typical_snowflake() {
        let id: u64 = 244_499_782_699_483_136;
        assert_eq!(as_u64(as_i64(id)), id);
    }

    #[test]
    fn round_trip_high_bit_set() {
        let id: u64 = u64::MAX;
        assert_eq!(as_u64(as_i64(id)), id);
    }
}
