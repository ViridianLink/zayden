use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    Mentionable,
    ResolvedValue,
    UserId,
};
use zayden_core::{InvocationCtx, as_i64, as_u64};

use crate::{FamilyError, FamilyRow, Result};

pub(in crate::commands) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "siblings",
        "List a user's siblings",
    )
    .add_sub_option(super::user_option(
        "The user to check. Leave blank to check yourself",
        false,
    ))
}

pub(in crate::commands) async fn run(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let user = super::target(&mut options, cx.interaction);

    let guild_id = cx.interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

    let row = FamilyRow::get(&cx.app.db, guild_id, user.id)
        .await?
        .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

    if row.parent_ids.is_empty() {
        return Err(no_siblings(user.id, user == &cx.interaction.user));
    }

    let user_id_signed: i64 = as_i64(user.id.get());

    let mut parent_rows = Vec::with_capacity(row.parent_ids.len());
    for parent_id in row.parent_ids {
        let parent_uid = UserId::new(as_u64(parent_id));
        if let Some(parent_row) =
            FamilyRow::get(&cx.app.db, guild_id, parent_uid).await?
        {
            parent_rows.push(parent_row);
        }
    }

    let mut siblings = Vec::new();
    for sib_id in collect_sibling_ids(&parent_rows, user_id_signed) {
        let sib_uid = UserId::new(as_u64(sib_id));
        let sib_user = sib_uid.to_user(cx.ctx).await?;
        siblings.push(sib_user.mention().to_string());
    }

    if siblings.is_empty() {
        return Err(no_siblings(user.id, user == &cx.interaction.user));
    }

    super::respond_list(cx, user.id, "siblings", &siblings).await
}

const fn no_siblings(user_id: UserId, is_self: bool) -> FamilyError {
    if is_self {
        FamilyError::SelfNoSiblings
    } else {
        FamilyError::NoSiblings(user_id)
    }
}

#[must_use]
pub fn collect_sibling_ids(parent_rows: &[FamilyRow], user_id: i64) -> Vec<i64> {
    parent_rows
        .iter()
        .flat_map(|row| row.children_ids.iter().copied())
        .filter(|id| *id != user_id)
        .collect()
}
