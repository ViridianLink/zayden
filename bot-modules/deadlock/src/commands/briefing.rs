use std::collections::HashMap;

use serenity::all::{
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ResolvedValue,
};
use sqlx::PgPool;
use zayden_core::{InvocationCtx, as_i64, optional_option};

use super::on_off;
use crate::db::{players, user_settings};
use crate::error::Result;

const LINK_STEAM: &str = "Link your Steam account with `/link steam` to get patch-day briefings for the heroes you play.";

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    pool: &PgPool,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let enabled: Option<bool> = optional_option(&mut options, "enabled");

    let user_id = as_i64(cx.interaction.user.id.get());

    if let Some(enabled) = enabled {
        user_settings::set_patch_briefing(pool, user_id, enabled).await?;
    }

    let mut lines = vec![format!(
        "Patch-day briefings: **{}**",
        on_off(user_settings::patch_briefing(pool, user_id).await?)
    )];

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
