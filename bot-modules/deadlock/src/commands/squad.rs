use serenity::all::{
    CreateAllowedMentions,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EditInteractionResponse,
    MessageFlags,
};
use sqlx::PgPool;
use zayden_core::{InvocationCtx, SubCommandOptions, as_i64, parse_subcommand};

use super::summaries::LINK_STEAM;
use crate::client::DeadlockApi;
use crate::db::{guild_members, players, squad_channel};
use crate::error::{DeadlockError, Result};
use crate::squad_deliver;

const JOINED: &str =
    "You joined this server's Deadlock squad. Rank changes and streaks post here.";
const NO_CHANNEL: &str = "An admin hasn't set a squad channel on the dashboard yet.";
const LEFT: &str = "You left this server's squad.";

async fn ephemeral(cx: &InvocationCtx<'_>, content: String) -> Result<()> {
    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(content),
            ),
        )
        .await?;

    Ok(())
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    pool: &PgPool,
    api: &DeadlockApi,
    options: SubCommandOptions<'_>,
) -> Result<()> {
    let (name, _) = parse_subcommand(options).map_err(DeadlockError::from)?;

    let Some(guild_id) = cx.interaction.guild_id.map(|guild| as_i64(guild.get()))
    else {
        return Err(DeadlockError::GuildOnly);
    };
    let user_id = as_i64(cx.interaction.user.id.get());

    match name {
        "join" => join(cx, pool, guild_id, user_id).await,
        "leave" => {
            guild_members::set_squad(pool, guild_id, user_id, false).await?;
            ephemeral(cx, LEFT.to_owned()).await
        },
        "leaderboard" => leaderboard(cx, pool, api, guild_id).await,
        _ => Err(DeadlockError::InvalidOption(name.to_string())),
    }
}

async fn join(
    cx: &InvocationCtx<'_>,
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<()> {
    if !players::is_linked(pool, user_id).await? {
        return ephemeral(cx, LINK_STEAM.to_owned()).await;
    }

    guild_members::set_squad(pool, guild_id, user_id, true).await?;

    let mut lines = vec![JOINED.to_owned()];
    if squad_channel::select(pool, guild_id).await?.is_none() {
        lines.push(NO_CHANNEL.to_owned());
    }

    ephemeral(cx, lines.join("\n")).await
}

async fn leaderboard(
    cx: &InvocationCtx<'_>,
    pool: &PgPool,
    api: &DeadlockApi,
    guild_id: i64,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let component = squad_deliver::leaderboard_message(pool, api, guild_id).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component])
                .allowed_mentions(CreateAllowedMentions::new()),
        )
        .await?;

    Ok(())
}
