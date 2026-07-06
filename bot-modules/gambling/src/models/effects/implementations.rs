use super::GamblingEffect;

pub(super) struct LuckyChipEffect;

impl GamblingEffect for LuckyChipEffect {
    fn id(&self) -> &'static str {
        "luckychip"
    }

    fn name(&self) -> &'static str {
        "Lucky Chip"
    }

    fn description(&self) -> &'static str {
        "Refund your bet if you lose"
    }

    fn on_loss(&self, bet: i64, _base_payout: i64) -> i64 {
        bet
    }
}

pub(super) struct AllInsEffect;

impl GamblingEffect for AllInsEffect {
    fn id(&self) -> &'static str {
        "allins"
    }

    fn name(&self) -> &'static str {
        "Infinite All Ins"
    }

    fn description(&self) -> &'static str {
        "Lets you go all-in above your max bet limit"
    }
}

pub(super) struct PayoutMultiplierEffect {
    id: &'static str,
    name: &'static str,
    multiplier: i64,
}

impl PayoutMultiplierEffect {
    pub(super) const fn new(
        id: &'static str,
        name: &'static str,
        multiplier: i64,
    ) -> Self {
        Self { id, name, multiplier }
    }
}

impl GamblingEffect for PayoutMultiplierEffect {
    fn id(&self) -> &'static str {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        "Multiplies your winning payouts"
    }

    fn on_win(&self, bet: i64, base_payout: i64) -> i64 {
        (base_payout - bet) * self.multiplier
    }
}
