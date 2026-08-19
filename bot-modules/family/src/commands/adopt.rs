use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Mentionable,
    ResolvedValue,
    User,
    UserId,
};
use zayden_core::InvocationCtx;

use crate::components::{ADOPT_ACCEPT, ADOPT_DECLINE};
use crate::relationships::Relationships;
use crate::{FamilyError, FamilyRow, Result};

pub(super) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "adopt",
        "Adopt another user into your family",
    )
    .add_sub_option(super::user_option("The user to adopt", true))
}

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let target_user: &User = super::required_user(&mut options, "user")?;

    let target_id = propose(cx, target_user).await?;

    let content = format!(
        "{}, {} wants to adopt you! Do you accept?",
        target_id.mention(),
        cx.interaction.user.mention()
    );

    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(content).components(
                    vec![super::proposal_buttons(ADOPT_ACCEPT, ADOPT_DECLINE)],
                ),
            ),
        )
        .await?;

    Ok(())
}

async fn propose(cx: &InvocationCtx<'_>, target_user: &User) -> Result<UserId> {
    let interaction = cx.interaction;

    if interaction.user.id == target_user.id {
        return Err(FamilyError::UserSelfAdopt);
    }

    if target_user.id == cx.ctx.http.get_current_user().await?.id {
        return Err(FamilyError::Zayden);
    }

    if target_user.bot() {
        return Err(FamilyError::Bot);
    }

    let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let adopter_row = FamilyRow::get(&cx.app.db, guild_id, interaction.user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, &interaction.user));

    if adopter_row.is_blocked(target_user.id) {
        return Err(FamilyError::Blocked(target_user.id));
    }

    if let Some(target_row) =
        FamilyRow::get(&cx.app.db, guild_id, target_user.id).await?
    {
        // Is already adopted?
        if !target_row.parent_ids.is_empty() {
            return Err(FamilyError::AlreadyAdopted(target_user.id));
        }

        if target_row.is_blocked(interaction.user.id) {
            return Err(FamilyError::Blocked(interaction.user.id));
        }
    }

    // Are the adopter and target are already related?
    let relationship = adopter_row.relationship(target_user.id);
    if relationship != Relationships::None {
        return Err(FamilyError::AlreadyRelated {
            target: target_user.id,
            relationship: Relationships::Parent,
        });
    }

    Ok(target_user.id)
}
