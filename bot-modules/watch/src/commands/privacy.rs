use std::collections::HashMap;
use std::hash::BuildHasher;

use jellyfin::identity::JellyfinLinkRow;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let visible: bool = required_option(&mut options, "visible")?;

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    JellyfinLinkRow::require(&cx.app.db, user_id).await?;
    JellyfinLinkRow::set_streak_public(&cx.app.db, user_id, visible).await?;

    let content = if visible {
        "Your watch streak is now public — anyone can run \
         `/watch streak` on you."
    } else {
        "Your watch streak is now private. Only you can see it."
    };

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
