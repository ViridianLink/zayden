use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    EditInteractionResponse,
    Mentionable,
    ResolvedValue,
    User,
};
use zayden_core::{InvocationCtx, optional_option};

use crate::{FamilyError, FamilyRow, Result};

pub(in crate::commands) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "relationship",
        "View the relationship between two users",
    )
    .add_sub_option(super::user_option(
        "The user you want to view the relationship of",
        true,
    ))
    .add_sub_option(CreateCommandOption::new(
        CommandOptionType::User,
        "other",
        "The other user. Leave blank to compare against yourself",
    ))
}

pub(in crate::commands) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let user: &User = super::super::required_user(&mut options, "user")?;

    let other: &User =
        optional_option(&mut options, "other").unwrap_or(&cx.interaction.user);

    if user == other {
        return Err(FamilyError::SameUser(user.id));
    }

    let guild_id = cx.interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let user_info = FamilyRow::get(&cx.app.db, guild_id, user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

    let relationship = user_info.relationship(other.id);

    let content = format!(
        "{} and {} are: **{relationship}**",
        user.id.mention(),
        other.id.mention(),
    );

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
