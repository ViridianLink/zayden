pub(super) mod children;
pub(super) mod parents;
pub(super) mod partner;
pub(super) mod relationship;
pub(super) mod siblings;

use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    Mentionable,
    ResolvedValue,
    User,
    UserId,
};
pub use siblings::collect_sibling_ids;
use zayden_core::{InvocationCtx, optional_option};

use super::user_option;
use crate::Result;

fn target<'a>(
    options: &mut HashMap<&str, ResolvedValue<'a>>,
    interaction: &'a CommandInteraction,
) -> &'a User {
    optional_option(options, "user").unwrap_or(&interaction.user)
}

async fn respond_list(
    cx: &InvocationCtx<'_>,
    user_id: UserId,
    label: &str,
    names: &[String],
) -> Result<()> {
    let content = format!("{}'s {label}: {}", user_id.mention(), names.join(", "));

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
