use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use futures::StreamExt;
use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    ButtonStyle, Colour, CommandInteraction, CommandOptionType, Context, CreateActionRow,
    CreateButton, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, EmojiId, ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::FormatNum;

use crate::events::{Dispatch, Event, GameEvent};
use crate::models::gambling::GamblingManager;
use crate::{
    CARD_BACK, CARD_DECK, COIN, Coins, EffectsManager, GameCache, GameManager, GameRow,
    GoalsManager, Result,
};

static CARD_TO_NUM: LazyLock<HashMap<EmojiId, u8>> = LazyLock::new(|| {
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

use super::Commands;

impl Commands {
    pub async fn blackjack<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(ctx).await.unwrap();

        let Some(ResolvedValue::Integer(mut bet)) = options.pop().map(|opt| opt.value) else {
            unreachable!("bet is required")
        };

        let mut tx = pool.begin().await.unwrap();

        let coins = GamblingHandler::coins(&mut *tx, interaction.user.id)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        GameCache::can_play(ctx, interaction.user.id).await?;
        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, coins).await?;
        GamblingHandler::bet(pool, interaction.user.id, bet)
            .await
            .unwrap();

        let mut card_shoe = CARD_DECK
            .iter()
            .copied()
            .cycle()
            .take(52 * 8)
            .collect::<Vec<_>>();

        card_shoe.shuffle(&mut rng());

        let mut player_hand = vec![card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        let mut player_value = sum_cards(&player_hand);
        let mut dealer_hand = vec![card_shoe.pop().unwrap(), card_shoe.pop().unwrap()];
        let mut dealer_value = sum_cards(&dealer_hand);

        /* TODO:
        - If the player's first two cards are an Ace and a 10-value card (10, J, Q, K), they have a "Blackjack" or a "Natural."
        - If the dealer is not showing an Ace or 10, the player wins immediately and is typically paid out at 3:2. (e.g., a 100-credit bet wins 150 credits).
        - If the dealer's upcard is an Ace or 10, they will check their hole card. If the dealer also has a Blackjack, the hand is a Push (a tie), and the player's bet is returned. If the dealer doesn't have Blackjack, the player wins and gets the 3:2 payout.
        */

        let embed = playing_embed(bet, &player_hand, player_value, &dealer_hand);

        let hit_btn = CreateButton::new("hit")
            .emoji('🎯')
            .label("Hit")
            .style(ButtonStyle::Secondary);
        let stand_btn = CreateButton::new("stand")
            .emoji('🛑')
            .label("Stand")
            .style(ButtonStyle::Secondary);
        let double_btn = CreateButton::new("double")
            .emoji('⏫')
            .label("Double Down")
            .style(ButtonStyle::Secondary)
            .disabled(coins < bet * 2);

        let message = interaction
            .edit_response(
                ctx,
                EditInteractionResponse::new().embed(embed).components(vec![
                    CreateActionRow::Buttons(vec![
                        hit_btn.clone(),
                        stand_btn.clone(),
                        double_btn.clone(),
                    ]),
                ]),
            )
            .await
            .unwrap();

        let mut stream = message
            .await_component_interactions(ctx)
            .author_id(interaction.user.id)
            .timeout(Duration::from_secs(120))
            .stream();

        while let Some(component) = stream.next().await {
            let custom_id = component.data.custom_id.as_str();

            match custom_id {
                "hit" => {
                    player_hand.push(card_shoe.pop().unwrap());
                }
                "stand" => break,
                "double" => {
                    GamblingHandler::bet(pool, interaction.user.id, bet)
                        .await
                        .unwrap();
                    bet *= 2;
                    player_hand.push(card_shoe.pop().unwrap());
                }
                _ => unreachable!("Invalid custom id"),
            };

            player_value = sum_cards(&player_hand);

            let embed = playing_embed(bet, &player_hand, player_value, &dealer_hand);

            component
                .create_response(
                    ctx,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new()
                            .embed(embed)
                            .components(vec![CreateActionRow::Buttons(vec![
                                hit_btn.clone(),
                                stand_btn.clone(),
                                double_btn.clone().disabled(true),
                            ])]),
                    ),
                )
                .await
                .unwrap();

            if player_value > 21 || custom_id == "double" {
                break;
            }
        }

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let dispatch = Dispatch::<Db, GoalsHandler>::new(ctx, pool);

        if player_value > 21 {
            dispatch
                .fire(
                    interaction.channel_id,
                    &mut row,
                    Event::Game(GameEvent::new("blackjack", interaction.user.id, bet, false)),
                )
                .await?;

            let payout =
                EffectsHandler::payout(pool, interaction.user.id, bet, 0, Some(false)).await;

            row.add_coins(payout);

            let coins = row.coins();

            GameHandler::save(pool, row).await.unwrap();
            GameCache::update(ctx, interaction.user.id).await;

            let desc = format!(
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
            );

            interaction
                .edit_response(
                    ctx,
                    EditInteractionResponse::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Blackjack - You Lost!")
                                .description(desc)
                                .colour(Colour::RED),
                        )
                        .components(Vec::new()),
                )
                .await
                .unwrap();

            return Ok(());
        }

        while dealer_value < 17 {
            dealer_hand.push(card_shoe.pop().unwrap());
            dealer_value = sum_cards(&dealer_hand);
        }

        let (win, mut payout) = if dealer_value > 21 || player_value > dealer_value {
            (Some(true), bet * 2)
        } else if player_value == dealer_value {
            (None, bet)
        } else {
            (Some(false), 0)
        };

        dispatch
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "blackjack",
                    interaction.user.id,
                    bet,
                    win == Some(true),
                )),
            )
            .await?;

        payout = EffectsHandler::payout(pool, interaction.user.id, bet, payout, win).await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(ctx, interaction.user.id).await;

        let desc = format!(
            "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}",
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
        );

        let embed = if win == Some(true) {
            CreateEmbed::new()
                .title("Blackjack - You Won!")
                .description(format!(
                    "{desc}\n\nProfit: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
                    (payout - bet).format(),
                    coins.format()
                ))
                .colour(Colour::DARK_GREEN)
        } else if win == Some(false) {
            CreateEmbed::new()
                            .title("Blackjack - You Lost!")
                            .description(format!("{desc}\n\nDealer wins!\n\nLost: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>", (payout - bet).format(),
                coins.format()))
                            .colour(Colour::RED)
        } else {
            CreateEmbed::new()
                .title("Blackjack - Draw!")
                .description(format!(
                    "{desc}\n\nDraw! Have your money back.\n\nYour coins: {} <:coin:{COIN}>",
                    coins.format()
                ))
                .colour(Colour::DARKER_GREY)
        };

        interaction
            .edit_response(
                ctx,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(Vec::new()),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_blackjack() -> CreateCommand {
        CreateCommand::new("blackjack")
            .description("Play a game of blackjack")
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                    .required(true),
            )
    }
}

fn sum_cards(hand: &[EmojiId]) -> u8 {
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

fn playing_embed(
    bet: i64,
    player_hand: &[EmojiId],
    player_value: u8,
    dealer_hand: &[EmojiId],
) -> CreateEmbed {
    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{}",
        bet.format(),
        player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .enumerate()
            .map(|(idx, (num, id))| if idx == 0 {
                format!("<:{num}:{id}> ")
            } else {
                format!("<:blank:{CARD_BACK}>")
            })
            .collect::<String>(),
    );

    CreateEmbed::new()
        .title("Blackjack")
        .description(desc)
        .colour(Colour::TEAL)
}
