use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ResolvedValue,
    User,
};
use zayden_core::InvocationCtx;

use crate::{FamilyError, FamilyRow, Result};

pub(super) fn register_block() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "block",
        "Block a user from being able to adopt/marry/etc you",
    )
    .add_sub_option(super::user_option("The user to block", true))
}

pub(super) fn register_unblock() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "unblock",
        "Unblock a user from being able to adopt/marry/etc you",
    )
    .add_sub_option(super::user_option("The user to unblock", true))
}

pub(super) async fn block(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let user: &User = super::required_user(&mut options, "user")?;

    let interaction = cx.interaction;

    if &interaction.user == user {
        return Err(FamilyError::UserSelfBlock);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let mut row = FamilyRow::get(&cx.app.db, guild_id, interaction.user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, &interaction.user));

    row.add_blocked(user.id);
    row.save(&cx.app.db).await?;

    respond(cx, "User blocked.").await
}

pub(super) async fn unblock(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let user: &User = super::required_user(&mut options, "user")?;

    let interaction = cx.interaction;

    if &interaction.user == user {
        return Err(FamilyError::UserSelfBlock);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    FamilyRow::remove_block(&cx.app.db, guild_id, interaction.user.id, user.id)
        .await?;

    respond(cx, "User unblocked.").await
}

async fn respond(cx: &InvocationCtx<'_>, content: &'static str) -> Result<()> {
    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}
