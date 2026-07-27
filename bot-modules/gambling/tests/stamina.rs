//! CC-3 regression: `Stamina::stamina_str` used to cast the raw `i32` stamina
//! straight to `usize` behind an `#[expect(clippy::cast_sign_loss)]`. Nothing
//! guarantees the value is non-negative — `done_work` decrements unconditionally
//! — and a negative cast wraps to ~2^64, so `str::repeat` aborts the process on
//! capacity overflow. The fix clamps into range before widening.

use gambling::Stamina;

struct Row(i32);

impl Stamina for Row {
    fn stamina(&self) -> i32 {
        self.0
    }

    fn stamina_mut(&mut self) -> &mut i32 {
        &mut self.0
    }
}

/// Every bar is `MAX_STAMINA` cells wide, whatever the input.
fn cell_count(bar: &str) -> usize {
    bar.chars().filter(|c| *c == '🟩' || *c == '⬛').count()
}

#[test]
fn full_stamina_is_all_filled() {
    let bar = Row(<Row as Stamina>::MAX_STAMINA).stamina_str();

    assert_eq!(cell_count(&bar), 3);
    assert_eq!(bar.matches('🟩').count(), 3);
    assert_eq!(bar.matches('⬛').count(), 0);
}

#[test]
fn partial_stamina_splits_filled_and_empty() {
    let bar = Row(1).stamina_str();

    assert_eq!(bar.matches('🟩').count(), 1);
    assert_eq!(bar.matches('⬛').count(), 2);
}

#[test]
fn zero_stamina_is_all_empty() {
    let bar = Row(0).stamina_str();

    assert_eq!(bar.matches('🟩').count(), 0);
    assert_eq!(bar.matches('⬛').count(), 3);
}

/// The regression itself: a negative stamina must render as an empty bar, not
/// abort trying to allocate ~2^64 copies of the filled cell.
#[test]
fn negative_stamina_renders_empty_instead_of_overflowing() {
    for stamina in [-1, -3, i32::MIN] {
        let bar = Row(stamina).stamina_str();

        assert_eq!(cell_count(&bar), 3, "stamina {stamina} changed the bar width");
        assert_eq!(bar.matches('🟩').count(), 0);
        assert_eq!(bar.matches('⬛').count(), 3);
    }
}

/// Stamina above the cap must not widen the bar either (`max - filled` would
/// underflow and panic if `filled` were left unclamped).
#[test]
fn over_cap_stamina_is_clamped_to_the_bar_width() {
    let bar = Row(i32::MAX).stamina_str();

    assert_eq!(cell_count(&bar), 3);
    assert_eq!(bar.matches('🟩').count(), 3);
    assert_eq!(bar.matches('⬛').count(), 0);
}
