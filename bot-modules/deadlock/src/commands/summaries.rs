use std::collections::HashMap;

use serenity::all::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::{InvocationCtx, as_i64, optional_option};

use crate::db::{guild_members, players, summary_channel, user_settings};
use crate::error::{DeadlockError, Result};

const NO_CHANNEL: &str = "This server has no summary channel yet — an admin can set one on the dashboard.";
const LINK_STEAM: &str = "Link your Steam account with `/link steam` to start receiving summaries. Summaries only appear once deadlock-api has your match; friending a deadlock-api bot keeps your history current.";

const fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let dm: Option<bool> = optional_option(&mut options, "dm");
    let server: Option<bool> = optional_option(&mut options, "server");

    let user_id = as_i64(cx.interaction.user.id.get());
    let guild_id = cx.interaction.guild_id.map(|guild| as_i64(guild.get()));

    if server.is_some() && guild_id.is_none() {
        return Err(DeadlockError::GuildOnly);
    }

    if let Some(enabled) = dm {
        user_settings::set_dm_summaries(pool, user_id, enabled).await?;
    }
    if let (Some(enabled), Some(guild_id)) = (server, guild_id) {
        guild_members::set_share_summaries(pool, guild_id, user_id, enabled).await?;
    }

    let mut lines = vec![format!(
        "DM summaries: **{}**",
        on_off(user_settings::dm_summaries(pool, user_id).await?)
    )];

    if let Some(guild_id) = guild_id {
        let shared = guild_members::share_summaries(pool, guild_id, user_id).await?;
        lines.push(format!("Sharing in this server: **{}**", on_off(shared)));

        if summary_channel::select(pool, guild_id).await?.is_none() {
            lines.push(NO_CHANNEL.to_owned());
        }
    }

    if !players::is_linked(pool, user_id).await? {
        lines.push(LINK_STEAM.to_owned());
    }

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(lines.join("\n")),
            ),
        )
        .await?;

    Ok(())
}
