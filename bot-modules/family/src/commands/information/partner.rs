use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt, stream};
use serenity::all::{
    CommandOptionType,
    CreateCommandOption,
    Mentionable,
    ResolvedValue,
    UserId,
};
use zayden_core::{InvocationCtx, as_u64};

use crate::{FamilyError, FamilyRow, Result};

pub(in crate::commands) fn register() -> CreateCommandOption<'static> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        "partner",
        "List who a user is married to",
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

    if row.partner_ids.is_empty() {
        if user == &cx.interaction.user {
            return Err(FamilyError::SelfNoPartners);
        }

        return Err(FamilyError::NoPartners(user.id));
    }

    let names: Vec<String> = stream::iter(row.partner_ids)
        .then(|id| async move {
            let user_id = UserId::new(as_u64(id));
            let user = user_id.to_user(cx.ctx).await?;

            Ok::<String, serenity::Error>(user.mention().to_string())
        })
        .try_collect()
        .await?;

    super::respond_list(cx, user.id, "partners", &names).await
}
