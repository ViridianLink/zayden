use gambling::{LottoRow, select_winners};

/// The three prize tiers the weekly draw pays out, mirroring `Lotto::cron_job`.
const PRIZE_SHARE: &[f64] = &[0.5, 0.3, 0.2];

const fn row(user_id: i64, quantity: i64) -> LottoRow {
    LottoRow { user_id, coins: 0, quantity: Some(quantity) }
}

/// DS-6 regression: with exactly as many participants as prize tiers, the final
/// pick empties the pool. The buggy code rebuilt `WeightedIndex` *after* that
/// last removal, hitting `WeightedIndex::new([])` -> `Err` and rolling the whole
/// draw back so nobody was paid. `select_winners` must instead return all three
/// winners.
#[test]
fn draws_all_winners_at_exactly_prize_tier_count() {
    let rows = vec![row(1, 5), row(2, 3), row(3, 2)];

    let jackpot = 1_000_000;
    let winners = select_winners(rows, PRIZE_SHARE, jackpot)
        .expect("a 3-participant draw must not roll back (DS-6)");

    assert_eq!(winners.len(), PRIZE_SHARE.len());

    // Each of the three participants wins exactly one tier (no double-wins).
    let mut ids: Vec<u64> = winners.iter().map(|(id, _)| id.get()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2, 3]);

    // Payouts are the jackpot split across the tiers, order-independent.
    let mut payouts: Vec<i64> = winners.iter().map(|(_, p)| *p).collect();
    payouts.sort_unstable();
    assert_eq!(payouts, vec![200_000, 300_000, 500_000]);
}

/// More participants than tiers still pays exactly one winner per tier.
#[test]
fn draws_one_winner_per_tier_with_surplus_participants() {
    let rows = vec![row(1, 5), row(2, 4), row(3, 3), row(4, 2), row(5, 1)];

    let winners =
        select_winners(rows, PRIZE_SHARE, 1_000_000).expect("surplus draw succeeds");

    assert_eq!(winners.len(), PRIZE_SHARE.len());

    let mut ids: Vec<u64> = winners.iter().map(|(id, _)| id.get()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), PRIZE_SHARE.len(), "winners must be distinct");
}
