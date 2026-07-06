use std::sync::LazyLock;

use super::GamblingEffect;
use super::implementations::{
    AllInsEffect,
    LuckyChipEffect,
    PayoutMultiplierEffect,
};

static EFFECTS: LazyLock<Vec<Box<dyn GamblingEffect>>> = LazyLock::new(|| {
    vec![
        Box::new(LuckyChipEffect),
        Box::new(AllInsEffect),
        Box::new(PayoutMultiplierEffect::new("payout2x", "Payout x2", 2)),
        Box::new(PayoutMultiplierEffect::new("payout5x", "Payout x5", 5)),
        Box::new(PayoutMultiplierEffect::new("payout10x", "Payout x10", 10)),
        Box::new(PayoutMultiplierEffect::new("payout50x", "Payout x50", 50)),
        Box::new(PayoutMultiplierEffect::new("payout100x", "Payout x100", 100)),
    ]
});

#[must_use]
pub fn get_effect(id: &str) -> Option<&'static dyn GamblingEffect> {
    LazyLock::force(&EFFECTS)
        .iter()
        .find(|effect| effect.id() == id)
        .map(AsRef::as_ref)
}
