use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{app_state, bearer_client, discord_client, guild_admin_context},
    std::collections::{HashMap, HashSet},
    std::sync::Arc,
    twilight_http::response::marker::ListBody,
    twilight_http::{Error, Response},
    twilight_model::application::command::Command,
    twilight_model::application::command::permissions::{
        CommandPermission,
        CommandPermissionType,
    },
    twilight_model::id::Id,
    twilight_model::id::marker::CommandMarker,
};

use crate::dto::ModuleView;

#[cfg(feature = "ssr")]
struct ModuleDef {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    commands: &'static [&'static str],
}

#[cfg(feature = "ssr")]
const MODULES: &[ModuleDef] = &[
    ModuleDef {
        id: "music",
        label: "Music",
        description: "Voice playback, queue, and 24/7 (Pro).",
        commands: &["music"],
    },
    ModuleDef {
        id: "palworld",
        label: "Palworld",
        description: "Save parsing, breeding solver, and world sync.",
        commands: &["palworld"],
    },
    ModuleDef {
        id: "marathon",
        label: "Marathon",
        description: "Marathon wiki lookups and news.",
        commands: &["marathon"],
    },
    ModuleDef {
        id: "gambling",
        label: "Gambling & Economy",
        description: "Currency games, shop, and leaderboards.",
        commands: &[
            "blackjack",
            "coinflip",
            "craft",
            "daily",
            "dig",
            "gift",
            "goals",
            "higherlower",
            "inventory",
            "leaderboard",
            "lotto",
            "mine",
            "prestige",
            "profile",
            "roll",
            "rockpaperscissors",
            "send",
            "shop",
            "tictactoe",
            "work",
        ],
    },
    ModuleDef {
        id: "family",
        label: "Family",
        description: "Marriage, adoption, and family tree commands.",
        commands: &[
            "marry",
            "divorce",
            "adopt",
            "block",
            "unblock",
            "children",
            "parents",
            "partner",
            "siblings",
            "relationship",
            "resetfamily",
            "tree",
        ],
    },
    ModuleDef {
        id: "ticket",
        label: "Tickets & Support",
        description: "Support tickets and FAQ panels.",
        commands: &["ticket", "support"],
    },
    ModuleDef {
        id: "misc",
        label: "Misc",
        description: "Miscellaneous utility commands.",
        commands: &["random", "custom_msg"],
    },
];

#[cfg(feature = "ssr")]
impl ModuleDef {
    fn view(
        &self,
        name_to_id: &HashMap<String, Id<CommandMarker>>,
        denied: &HashSet<Id<CommandMarker>>,
    ) -> ModuleView {
        let known: Vec<_> =
            self.commands.iter().filter_map(|c| name_to_id.get(*c)).collect();
        let enabled =
            known.is_empty() || known.iter().any(|id| !denied.contains(*id));

        ModuleView {
            id: self.id.to_string(),
            label: self.label.to_string(),
            description: self.description.to_string(),
            commands: self.commands.iter().map(|c| (*c).to_string()).collect(),
            enabled,
        }
    }
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
async fn fetch_command_ids(
    http: &twilight_http::Client,
    app_id: u64,
    guild_id: u64,
) -> HashMap<String, Id<CommandMarker>> {
    let interaction = http.interaction(Id::new(app_id));
    let (global, guild) = tokio::join!(
        interaction.global_commands(),
        interaction.guild_commands(Id::new(guild_id)),
    );

    let mut merged = command_ids(global).await;
    merged.extend(command_ids(guild).await);
    merged
}

#[cfg(feature = "ssr")]
struct GuildContext {
    guild_id: u64,
    access_token: String,
    http: Arc<twilight_http::Client>,
    app_id: u64,
}

#[cfg(feature = "ssr")]
async fn guild_context(guild: &str) -> Result<GuildContext, ServerFnError> {
    let (guild_id, _user, access_token) = guild_admin_context(guild).await?;

    Ok(GuildContext {
        guild_id: guild_id.cast_unsigned(),
        access_token,
        http: discord_client()?,
        app_id: app_state()?.zayden_id,
    })
}

#[cfg(feature = "ssr")]
async fn denied_commands(
    access_token: &str,
    app_id: u64,
    guild_id: u64,
) -> HashSet<Id<CommandMarker>> {
    let resp = bearer_client(access_token)
        .interaction(Id::new(app_id))
        .guild_command_permissions(Id::new(guild_id))
        .await;
    let Ok(resp) = resp else {
        return HashSet::new();
    };

    resp.models()
        .await
        .map(|list| {
            list.into_iter()
                .filter(|cp| {
                    cp.permissions.iter().any(|p| {
                        !p.permission
                            && matches!(
                                &p.id,
                                CommandPermissionType::Role(role_id)
                                    if role_id.get() == guild_id
                            )
                    })
                })
                .map(|cp| cp.id)
                .collect()
        })
        .unwrap_or_default()
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    let ctx = guild_context(&guild).await?;

    let name_to_id = fetch_command_ids(&ctx.http, ctx.app_id, ctx.guild_id).await;
    let denied = denied_commands(&ctx.access_token, ctx.app_id, ctx.guild_id).await;

    Ok(MODULES.iter().map(|m| m.view(&name_to_id, &denied)).collect())
}

#[server]
pub async fn set_module_enabled(
    guild: String,
    module_id: String,
    enabled: bool,
) -> Result<(), ServerFnError> {
    let Some(module) = MODULES.iter().find(|m| m.id == module_id) else {
        return Err(ServerFnError::ServerError("unknown module".to_string()));
    };

    let ctx = guild_context(&guild).await?;
    let name_to_id = fetch_command_ids(&ctx.http, ctx.app_id, ctx.guild_id).await;

    let permissions: Vec<CommandPermission> = if enabled {
        Vec::new()
    } else {
        vec![CommandPermission {
            id: CommandPermissionType::Role(Id::new(ctx.guild_id)),
            permission: false,
        }]
    };

    let bearer = bearer_client(&ctx.access_token);
    let interaction = bearer.interaction(Id::new(ctx.app_id));
    for name in module.commands {
        let Some(cmd_id) = name_to_id.get(*name) else {
            continue;
        };
        if let Err(e) = interaction
            .update_command_permissions(Id::new(ctx.guild_id), *cmd_id, &permissions)
            .await
        {
            return Err(ServerFnError::ServerError(format!(
                "Discord rejected the permission update for /{name}: {e}"
            )));
        }
    }

    Ok(())
}
