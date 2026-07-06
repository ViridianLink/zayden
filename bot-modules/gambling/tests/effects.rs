use gambling::SHOP_ITEMS;
use gambling::models::get_effect;

const EFFECT_IDS: &[&str] = &[
    "luckychip",
    "allins",
    "payout2x",
    "payout5x",
    "payout10x",
    "payout50x",
    "payout100x",
];

#[test]
fn lucky_chip_refunds_bet_on_loss_only() {
    let lucky_chip = get_effect("luckychip").expect("luckychip is registered");

    // Lucky Chip returns the full stake on a loss and nothing on a win.
    assert_eq!(lucky_chip.on_loss(500, 0), 500);
    assert_eq!(lucky_chip.on_win(500, 1000), 0);
}

#[test]
fn all_ins_has_no_payout_contribution() {
    let all_ins = get_effect("allins").expect("allins is registered");

    // All Ins carries no payout hooks; its limit-lifting lives in
    // `EffectsManager::bet_limit`, not in the trait.
    assert_eq!(all_ins.on_win(500, 1000), 0);
    assert_eq!(all_ins.on_loss(500, 0), 0);
}

#[test]
fn payout_multiplier_scales_winnings() {
    let payout5x = get_effect("payout5x").expect("payout5x is registered");

    // bet 100, base payout 200 => winnings 100, scaled x5 => 500.
    assert_eq!(payout5x.on_win(100, 200), 500);
}

#[test]
fn payout_multiplier_neutral_on_loss() {
    let payout2x = get_effect("payout2x").expect("payout2x is registered");

    // Multipliers only ever contribute on a win.
    assert_eq!(payout2x.on_loss(500, 0), 0);
}

#[test]
fn get_effect_resolves_each_registered_id() {
    for id in EFFECT_IDS {
        let resolved =
            get_effect(id).expect("registered effect should resolve by its id");
        assert_eq!(resolved.id(), *id);
    }
}

#[test]
fn get_effect_returns_none_for_unknown_id() {
    assert!(get_effect("does-not-exist").is_none());
}

#[test]
fn every_effect_id_matches_a_shop_item() {
    for id in EFFECT_IDS {
        assert!(
            SHOP_ITEMS.iter().any(|item| item.id == *id),
            "effect id '{id}' has no matching ShopItem"
        );
    }
}
