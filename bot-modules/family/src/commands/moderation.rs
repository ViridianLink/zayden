use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Permissions,
};
use zayden_core::InvocationCtx;

use crate::{FamilyError, FamilyRow, Result};

pub(super) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "reset",
        "Reset every family tree in this server (Administrator)",
    )
}

pub(super) async fn run(cx: &InvocationCtx<'_>) -> Result<()> {
    let guild_id = cx.interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let privileged = cx
        .interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
        .is_some_and(Permissions::administrator);

    if !privileged {
        return Err(FamilyError::NotPrivileged);
    }

    FamilyRow::reset(&cx.app.db, guild_id).await?;

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Family trees have been reset.")
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}
