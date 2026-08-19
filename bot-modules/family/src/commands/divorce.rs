use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Mentionable,
    ResolvedValue,
    User,
};
use zayden_core::{InvocationCtx, as_i64};

use crate::{FamilyError, FamilyRow, Result};

pub(super) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "divorce",
        "Divorce your partner",
    )
    .add_sub_option(super::user_option("The partner to divorce", true))
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let target_user: &User = super::required_user(&mut options, "user")?;

    let interaction = cx.interaction;

    if interaction.user.id == target_user.id {
        return Err(FamilyError::UserSelfMarry);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let row = FamilyRow::get(&cx.app.db, guild_id, interaction.user.id)
        .await?
        .ok_or(FamilyError::SelfNoPartners)?;

    if !row.partner_ids.contains(&as_i64(target_user.id.get())) {
        return Err(FamilyError::NotPartners(target_user.id));
    }

    FamilyRow::remove_partner(
        &cx.app.db,
        guild_id,
        interaction.user.id,
        target_user.id,
    )
    .await?;

    let content = format!("You have divorced {}.", target_user.id.mention());

    interaction
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
