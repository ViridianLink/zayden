use std::collections::HashMap;

use serenity::all::{
    EditInteractionResponse,
    GenericInteractionChannel,
    Permissions,
    ResolvedValue,
};
use zayden_core::{
    InvocationCtx,
    SubCommandOptions,
    as_i64,
    parse_options,
    parse_subcommand,
    required_option,
};

use crate::announce::MarathonAnnounceRow;
use crate::error::{MarathonError, Result};

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    sub_options: SubCommandOptions<'_>,
) -> Result<()> {
    let (name, options) =
        parse_subcommand(sub_options).map_err(MarathonError::from)?;
    let options = parse_options(options);

    match name {
        "set" => set(cx, options).await,
        "disable" => disable(cx).await,
        _ => Err(MarathonError::NotFound {
            entity: "subcommand",
            query: name.to_string(),
        }),
    }
}

fn require_manage_guild(cx: &InvocationCtx<'_>) -> Result<()> {
    let privileged = cx
        .interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
        .is_some_and(Permissions::manage_guild);

    if privileged { Ok(()) } else { Err(MarathonError::NotPrivileged) }
}

async fn set(
    cx: &InvocationCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let guild_id = cx.interaction.guild_id.ok_or(MarathonError::MissingGuildId)?;
    require_manage_guild(cx)?;

    let channel: &GenericInteractionChannel =
        required_option(&mut options, "channel").map_err(MarathonError::from)?;
    let channel_id = channel.id().expect_channel();

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    MarathonAnnounceRow::upsert(
        &cx.app.db,
        as_i64(guild_id.get()),
        as_i64(channel_id.get()),
    )
    .await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().content(format!(
                "Marathon schedule announcements will be posted in <#{channel_id}>."
            )),
        )
        .await?;

    Ok(())
}

async fn disable(cx: &InvocationCtx<'_>) -> Result<()> {
    let guild_id = cx.interaction.guild_id.ok_or(MarathonError::MissingGuildId)?;
    require_manage_guild(cx)?;

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    MarathonAnnounceRow::delete(&cx.app.db, as_i64(guild_id.get())).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new()
                .content("Marathon schedule announcements disabled."),
        )
        .await?;

    Ok(())
}
