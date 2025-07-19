use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use futures::StreamExt;
use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CollectComponentInteractions, Colour, CommandInteraction, ComponentInteraction, Context,
    CreateButton, CreateCommand, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, EmojiId, Http, parse_emoji,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::FormatNum;

use crate::events::{Dispatch, Event, GameEvent};
use crate::models::gambling_stats::StatsManager;
use crate::{CARD_DECK, Coins, GamblingData, GameCache, GameManager, Gems, GoalsManager, Result};

use super::Commands;

static CARD_TO_NUM: LazyLock<HashMap<EmojiId, u8>> = LazyLock::new(|| {
    CARD_DECK
        .iter()
        .copied()
        .zip((1u8..=13).cycle().take(52))
        .collect()
});

impl Commands {
    pub async fn higher_lower<
        Data: GamblingData,
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        GameHandler: GameManager<Db>,
        StatsHandler: StatsManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let data = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data), interaction.user.id).await?;

        let mut deck = CARD_DECK.to_vec();
        deck.shuffle(&mut rng());

        let emoji = deck.pop().unwrap();
        let num = CARD_TO_NUM.get(&emoji).unwrap();

        let embed = create_embed(&format!("<:{num}:{emoji}>"), 0, true);

        let higher_btn = CreateButton::new("hol_higher").emoji('☝').label("Higher");
        let lower_btn = CreateButton::new("hol_lower").emoji('👇').label("Lower");

        let msg = interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .button(higher_btn)
                    .button(lower_btn),
            )
            .await
            .unwrap();

        let mut stream = msg
            .id
            .collect_component_interactions(ctx)
            .author_id(interaction.user.id)
            .timeout(Duration::from_secs(120))
            .stream();

        let mut payout = 0;
        let mut prev_seq = String::new();

        while let Some(interaction) = stream.next().await {
            let mut desc_iter = interaction
                .message
                .embeds
                .first()
                .unwrap()
                .description
                .as_deref()
                .unwrap()
                .split("\n\n");

            prev_seq = desc_iter.next().unwrap().to_string();
            let prev_emoji = parse_emoji(prev_seq.split(' ').next_back().unwrap()).unwrap();
            let prev_num = prev_emoji.name.parse::<u8>().unwrap();

            let emoji = match deck.pop() {
                Some(emoji) => emoji,
                None => break,
            };
            let num = *CARD_TO_NUM.get(&emoji).unwrap();

            payout = desc_iter
                .next()
                .unwrap()
                .strip_prefix("Current Payout: ")
                .unwrap()
                .replace(',', "")
                .parse::<i64>()
                .unwrap();

            let choice = interaction.data.custom_id.as_str();

            let winner = if choice == "hol_higher" {
                higher(
                    &ctx.http,
                    &interaction,
                    &mut prev_seq,
                    prev_num,
                    num,
                    emoji,
                    payout,
                )
                .await?
            } else {
                lower(
                    &ctx.http,
                    &interaction,
                    &mut prev_seq,
                    prev_num,
                    num,
                    emoji,
                    payout,
                )
                .await?
            };

            if !winner {
                break;
            }
        }

        let mut tx = pool.begin().await.unwrap();

        StatsHandler::higherlower(&mut *tx, interaction.user.id, (payout / 1000) as i32)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap();

        row.add_coins(payout);

        if payout == 52_000 {
            row.add_gems(1);
        }

        let colour = if payout > 0 {
            Colour::DARK_GREEN
        } else {
            Colour::RED
        };

        let coins = row.coins_str();

        Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "higherorlower",
                    interaction.user.id,
                    payout,
                    payout != 0,
                )),
            )
            .await?;

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(data, interaction.user.id).await;

        let result = format!("Payout: {}", payout.format());

        let embed = CreateEmbed::new()
            .title("Higher or Lower")
            .description(format!(
                "{}\n\nFinal Payout: {}\n\nThis game has ended.\n\n{result}\nYour coins: {coins}",
                prev_seq,
                payout.format()
            ))
            .colour(colour);

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(Vec::new()),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_higher_lower<'a>() -> CreateCommand<'a> {
        CreateCommand::new("higherorlower").description("Play a game of higher or lower")
    }
}

fn create_embed<'a>(seq: &str, payout: i64, winner: bool) -> CreateEmbed<'a> {
    let payout = payout.format();

    let desc = if winner {
        format!("{seq}\n\nCurrent Payout: {payout}\n\nGuess the next number!")
    } else {
        format!("{seq}\n\nFinal Payout: {payout}")
    };

    CreateEmbed::new()
        .title("Higher or Lower")
        .description(desc)
        .colour(Colour::TEAL)
}

async fn higher(
    http: &Http,
    interaction: &ComponentInteraction,
    seq: &mut String,
    prev: u8,
    next: u8,
    emoji: EmojiId,
    mut payout: i64,
) -> Result<bool> {
    seq.push(' ');

    let winner = next >= prev;

    if winner {
        seq.push('☝');
        payout += 1000
    } else {
        seq.push('❌');
    }

    seq.push_str(&format!(" <:{next}:{emoji}>"));

    let embed = create_embed(seq, payout, winner);

    let msg = if winner {
        CreateInteractionResponseMessage::new().embed(embed)
    } else {
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(Vec::new())
    };

    interaction
        .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
        .await?;

    Ok(winner)
}

async fn lower(
    http: &Http,
    interaction: &ComponentInteraction,
    seq: &mut String,
    prev: u8,
    next: u8,
    emoji: EmojiId,
    mut payout: i64,
) -> Result<bool> {
    seq.push(' ');

    let winner = next <= prev;

    if winner {
        seq.push('👇');
        payout += 1000
    } else {
        seq.push('❌');
    }

    seq.push_str(&format!(" <:{next}:{emoji}>"));

    let embed = create_embed(seq, payout, winner);

    let msg = if winner {
        CreateInteractionResponseMessage::new().embed(embed)
    } else {
        CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(Vec::new())
    };

    interaction
        .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
        .await?;

    Ok(winner)
}
