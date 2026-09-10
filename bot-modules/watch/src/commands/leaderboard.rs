use std::collections::HashMap;
use std::hash::BuildHasher;

use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option};

use crate::embeds::COLOUR;
use crate::error::{Result, WatchError};
use crate::games::score::ScoreRow;

const TOP_N: i64 = 10;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let game: Option<&str> = optional_option(&mut options, "game");
    let game = game.filter(|g| *g != "all");

    let guild_id = cx.interaction.guild_id.ok_or(WatchError::MissingGuildId)?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let rows = ScoreRow::leaderboard(&cx.app.db, guild_id, game, TOP_N).await?;

    let body = if rows.is_empty() {
        "Nobody has played yet.".to_owned()
    } else {
        rows.iter()
            .enumerate()
            .map(|(i, row)| {
                format!(
                    "**{}.** <@{}> — {} points ({}/{})",
                    i + 1,
                    row.user(),
                    row.points,
                    row.correct,
                    row.played
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let title = game.map_or_else(
        || "Leaderboard".to_owned(),
        |name| format!("Leaderboard: {name}"),
    );

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new().title(title).colour(COLOUR).description(body),
            ),
        )
        .await?;

    Ok(())
}
