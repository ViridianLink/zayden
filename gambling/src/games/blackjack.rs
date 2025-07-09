use std::{collections::HashMap, sync::LazyLock};

use serenity::all::{ButtonStyle, Colour, CreateButton, CreateEmbed, EmojiId};
use zayden_core::FormatNum;

use crate::{CARD_BACK, CARD_DECK, COIN};

pub static CARD_TO_NUM: LazyLock<HashMap<EmojiId, u8>> = LazyLock::new(|| {
    CARD_DECK
        .iter()
        .copied()
        .zip(
            (1u8..=13)
                .cycle()
                .map(|rank| match rank {
                    11..=13 => 10,
                    _ => rank,
                })
                .take(52),
        )
        .collect()
});

pub fn sum_cards(hand: &[EmojiId]) -> u8 {
    let (aces, rest) = hand
        .iter()
        .map(|id| *CARD_TO_NUM.get(id).unwrap())
        .partition::<Vec<_>, _>(|num| *num == 1);

    let mut sum = rest.iter().sum();
    let mut num_aces = aces.len();

    sum += num_aces as u8 * 11;

    while sum > 21 && num_aces > 0 {
        sum -= 10;
        num_aces -= 1;
    }

    sum
}

pub fn in_play_embed(bet: i64, player_hand: &[EmojiId], dealer_card: EmojiId) -> CreateEmbed {
    let player_value = sum_cards(player_hand);
    let dealer_value = sum_cards(&[dealer_card]);

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n<:{}:{dealer_card}> <:blank:{CARD_BACK}> - {dealer_value}",
        bet.format(),
        player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        CARD_TO_NUM.get(&dealer_card).unwrap(),
    );

    CreateEmbed::new()
        .title("Blackjack")
        .description(desc)
        .colour(Colour::TEAL)
}

pub fn hit_button() -> CreateButton {
    CreateButton::new("blackjack_hit")
        .emoji('🎯')
        .label("Hit")
        .style(ButtonStyle::Secondary)
}

pub fn stand_button() -> CreateButton {
    CreateButton::new("blackjack_stand")
        .emoji('🛑')
        .label("Stand")
        .style(ButtonStyle::Secondary)
}

pub fn double_button(coins: i64, bet: i64) -> CreateButton {
    CreateButton::new("blackjack_double")
        .emoji('⏫')
        .label("Double Down")
        .style(ButtonStyle::Secondary)
        .disabled(coins < bet * 2)
}

pub fn game_end_desc(
    bet: i64,
    player_hand: &[EmojiId],
    dealer_hand: &[EmojiId],
    payout: i64,
    coins: i64,
) -> String {
    let player_value = sum_cards(player_hand);
    let dealer_value = sum_cards(dealer_hand);

    format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
        bet.format(),
        player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        (payout - bet).format(),
        coins.format()
    )
}
