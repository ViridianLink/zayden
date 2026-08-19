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
use zayden_core::InvocationCtx;

use crate::components::{MARRY_ACCEPT, MARRY_DECLINE};
use crate::relationships::Relationships;
use crate::{FamilyError, FamilyRow, FamilySettings, Result};

pub(super) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "marry",
        "Propose to another Discord user",
    )
    .add_sub_option(super::user_option("The user you want to propose to", true))
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let target_user: &User = super::required_user(&mut options, "user")?;

    let target_id = propose(cx, target_user).await?;

    let content = format!(
        "{}, {} wants to marry you! Do you accept?",
        target_id.mention(),
        cx.interaction.user.mention()
    );

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content).components(
                    vec![super::proposal_buttons(MARRY_ACCEPT, MARRY_DECLINE)],
                ),
            ),
        )
        .await?;

    Ok(())
}

async fn propose(
    cx: &InvocationCtx<'_>,
    target_user: &User,
) -> Result<serenity::all::UserId> {
    let interaction = cx.interaction;

    if interaction.user.id == target_user.id {
        return Err(FamilyError::UserSelfMarry);
    }

    if target_user.id == cx.ctx.http.get_current_user().await?.id {
        return Err(FamilyError::Zayden);
    }

    if target_user.bot() {
        return Err(FamilyError::Bot);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let max_partners =
        FamilySettings::get(&cx.app.db, guild_id).await?.max_partners();

    if let Some(row) =
        FamilyRow::get(&cx.app.db, guild_id, interaction.user.id).await?
    {
        let relationship = row.relationship(target_user.id);

        if relationship != Relationships::None {
            return Err(FamilyError::AlreadyRelated {
                target: target_user.id,
                relationship,
            });
        }

        if row.at_partner_limit(max_partners) {
            return Err(FamilyError::MaxPartners);
        }

        if row.is_blocked(target_user.id) {
            return Err(FamilyError::Blocked(target_user.id));
        }
    }

    if let Some(row) = FamilyRow::get(&cx.app.db, guild_id, target_user.id).await? {
        if row.at_partner_limit(max_partners) {
            return Err(FamilyError::MaxPartners);
        }

        if row.is_blocked(interaction.user.id) {
            return Err(FamilyError::Blocked(interaction.user.id));
        }
    }

    Ok(target_user.id)
}
