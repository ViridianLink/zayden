use twilight_model::application::command::permissions::{
    CommandPermission,
    CommandPermissionType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker, RoleMarker};
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{
        app_state,
        bearer_client,
        discord_client,
        guild_admin_context,
    },
    leptos::prelude::ServerFnError,
    std::collections::HashMap,
    std::sync::Arc,
    twilight_http::response::marker::ListBody,
    twilight_http::{Error, Response},
    twilight_model::application::command::Command,
    twilight_model::id::marker::CommandMarker,
};

/// Discord caps a command at 100 overwrites. One is spent on the "deny every
/// channel" sentinel and one may be spent on the module switch, so the
/// allowlist stops short of the limit rather than at it.
pub const MAX_ALLOWED_CHANNELS: usize = 90;

/// Discord's sentinel for "every channel in this guild": the `@everyone` role
/// id — which equals the guild id — minus one.
///
/// Returns `None` only for a guild id of 1, which no real snowflake is.
#[must_use]
pub const fn all_channels(guild_id: Id<GuildMarker>) -> Option<Id<ChannelMarker>> {
    Id::new_checked(guild_id.get().saturating_sub(1))
}

/// The `@everyone` role id, which Discord defines as the guild id.
#[must_use]
pub const fn everyone(guild_id: Id<GuildMarker>) -> Id<RoleMarker> {
    guild_id.cast()
}

/// Whether `permissions` denies the command to `@everyone` — the modules page's
/// on/off switch.
#[must_use]
pub fn everyone_denied(
    guild_id: Id<GuildMarker>,
    permissions: &[CommandPermission],
) -> bool {
    permissions.iter().any(|p| {
        !p.permission
            && matches!(p.id, CommandPermissionType::Role(role) if role == everyone(guild_id))
    })
}

/// Rewrite the `@everyone` entry, leaving every channel and user entry in
/// place.
///
/// Without the preservation, turning a module off and on again would silently
/// discard the channel restriction configured on its own settings page.
#[must_use]
pub fn with_everyone_denied(
    guild_id: Id<GuildMarker>,
    permissions: &[CommandPermission],
    denied: bool,
) -> Vec<CommandPermission> {
    let everyone = everyone(guild_id);

    let mut out = permissions
        .iter()
        .filter(|p| !matches!(p.id, CommandPermissionType::Role(role) if role == everyone))
        .cloned()
        .collect::<Vec<_>>();

    if denied {
        out.push(CommandPermission {
            id: CommandPermissionType::Role(everyone),
            permission: false,
        });
    }

    out
}

/// The channels the command is limited to, or empty for "anywhere".
///
/// The individual allows only mean anything alongside the deny-all sentinel;
/// without it the command already runs everywhere, so the list is reported as
/// empty rather than as a restriction that is not in force.
#[must_use]
pub fn channel_allowlist(
    guild_id: Id<GuildMarker>,
    permissions: &[CommandPermission],
) -> Vec<Id<ChannelMarker>> {
    let Some(all) = all_channels(guild_id) else {
        return Vec::new();
    };

    let restricted = permissions.iter().any(|p| {
        !p.permission
            && matches!(p.id, CommandPermissionType::Channel(channel) if channel == all)
    });

    if !restricted {
        return Vec::new();
    }

    permissions
        .iter()
        .filter_map(|p| match p.id {
            CommandPermissionType::Channel(channel)
                if p.permission && channel != all =>
            {
                Some(channel)
            },
            CommandPermissionType::Channel(_)
            | CommandPermissionType::Role(_)
            | CommandPermissionType::User(_) => None,
        })
        .collect()
}

/// Rewrite the channel entries so the command is limited to `allowlist`,
/// leaving role and user entries in place.
///
/// An empty `allowlist` drops the restriction entirely — including the deny-all
/// sentinel — rather than leaving a command nobody can run anywhere.
#[must_use]
pub fn with_channel_allowlist(
    guild_id: Id<GuildMarker>,
    permissions: &[CommandPermission],
    allowlist: &[Id<ChannelMarker>],
) -> Vec<CommandPermission> {
    let mut out = permissions
        .iter()
        .filter(|p| !matches!(p.id, CommandPermissionType::Channel(_)))
        .cloned()
        .collect::<Vec<_>>();

    let Some(all) = all_channels(guild_id) else {
        return out;
    };

    if allowlist.is_empty() {
        return out;
    }

    out.push(CommandPermission {
        id: CommandPermissionType::Channel(all),
        permission: false,
    });

    for channel in allowlist {
        if *channel == all {
            continue;
        }

        out.push(CommandPermission {
            id: CommandPermissionType::Channel(*channel),
            permission: true,
        });
    }

    out
}

/// The authenticated context every permission read or write needs.
///
/// Writes go through the admin's own OAuth token rather than the bot token,
/// because Discord only accepts command-permission updates on a Bearer
/// credential belonging to someone who could make the change by hand.
#[cfg(feature = "ssr")]
pub(crate) struct GuildContext {
    pub(crate) guild_id: Id<GuildMarker>,
    pub(crate) access_token: String,
    pub(crate) http: Arc<twilight_http::Client>,
    pub(crate) app_id: u64,
}

#[cfg(feature = "ssr")]
pub(crate) async fn guild_context(
    guild: &str,
) -> Result<GuildContext, ServerFnError> {
    let (guild_id, _user, access_token) = guild_admin_context(guild).await?;

    Ok(GuildContext {
        guild_id: Id::new(guild_id.cast_unsigned()),
        access_token,
        http: discord_client()?,
        app_id: app_state()?.zayden_id,
    })
}

#[cfg(feature = "ssr")]
async fn command_ids(
    list: Result<Response<ListBody<Command>>, Error>,
) -> HashMap<String, Id<CommandMarker>> {
    let Ok(resp) = list else {
        return HashMap::new();
    };
    resp.models()
        .await
        .map(|cmds| {
            cmds.into_iter().filter_map(|c| c.id.map(|id| (c.name, id))).collect()
        })
        .unwrap_or_default()
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_command_ids(
    ctx: &GuildContext,
) -> HashMap<String, Id<CommandMarker>> {
    let interaction = ctx.http.interaction(Id::new(ctx.app_id));
    let (global, guild) = tokio::join!(
        interaction.global_commands(),
        interaction.guild_commands(ctx.guild_id),
    );

    let mut merged = command_ids(global).await;
    merged.extend(command_ids(guild).await);
    merged
}

#[cfg(feature = "ssr")]
pub(crate) async fn command_id(
    ctx: &GuildContext,
    name: &str,
) -> Result<Id<CommandMarker>, ServerFnError> {
    fetch_command_ids(ctx).await.get(name).copied().ok_or_else(|| {
        ServerFnError::ServerError(format!(
            "/{name} isn't registered for this server yet"
        ))
    })
}

/// The overwrites currently set on one command.
///
/// Discord answers 404 when a command has never had any, which is not an error
/// — it is the empty list.
#[cfg(feature = "ssr")]
pub(crate) async fn fetch(
    ctx: &GuildContext,
    command: Id<CommandMarker>,
) -> Vec<CommandPermission> {
    let resp = bearer_client(&ctx.access_token)
        .interaction(Id::new(ctx.app_id))
        .command_permissions(ctx.guild_id, command)
        .await;

    let Ok(resp) = resp else {
        return Vec::new();
    };

    resp.model().await.map(|p| p.permissions).unwrap_or_default()
}

#[cfg(feature = "ssr")]
pub(crate) async fn store(
    ctx: &GuildContext,
    command: Id<CommandMarker>,
    name: &str,
    permissions: &[CommandPermission],
) -> Result<(), ServerFnError> {
    bearer_client(&ctx.access_token)
        .interaction(Id::new(ctx.app_id))
        .update_command_permissions(ctx.guild_id, command, permissions)
        .await
        .map(|_resp| ())
        .map_err(|e| {
            ServerFnError::ServerError(format!(
                "Discord rejected the permission update for /{name}: {e}"
            ))
        })
}
