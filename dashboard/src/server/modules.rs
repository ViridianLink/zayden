use leptos::prelude::*;
#[cfg(feature = "ssr")]
use twilight_model::id::{Id, marker::CommandMarker};

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
async fn fetch_command_ids(
    http: &reqwest::Client,
    app_id: u64,
    bot_token: &str,
) -> std::collections::HashMap<String, Id<CommandMarker>> {
    use std::collections::HashMap;

    use twilight_model::application::command::Command;

    let resp = http
        .get(format!("https://discord.com/api/v10/applications/{app_id}/commands"))
        .header("Authorization", format!("Bot {bot_token}"))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => r
            .json::<Vec<Command>>()
            .await
            .map(|cmds| {
                cmds.into_iter()
                    .filter_map(|c| c.id.map(|id| (c.name, id)))
                    .collect()
            })
            .unwrap_or_default(),
        _ => HashMap::new(),
    }
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    use std::collections::HashSet;
    use std::sync::Arc;

    use reqwest::Client;
    use zayden_app::state::AppState;

    use crate::app::DiscordBotToken;
    use crate::server::auth::guild_admin_context;

    let (guild_id_i64, _user_id, access_token) = guild_admin_context(&guild).await?;
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let bot_token =
        use_context::<DiscordBotToken>().map(|t| t.0).unwrap_or_default();
    let app_id = app.zayden_id;

    let name_to_id = fetch_command_ids(&http, app_id, &bot_token).await;

    let denied: HashSet<Id<CommandMarker>> = {
        use twilight_model::application::command::permissions::{
            CommandPermissionType,
            GuildCommandPermissions,
        };

        let url = format!(
            "https://discord.com/api/v10/applications/{app_id}/guilds/{guild_id_u64}/commands/permissions"
        );
        let resp = http
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => r
                .json::<Vec<GuildCommandPermissions>>()
                .await
                .map(|list| {
                    list.into_iter()
                        .filter(|cp| {
                            cp.permissions.iter().any(|p| {
                                !p.permission
                                    && matches!(
                                        &p.id,
                                        CommandPermissionType::Role(role_id)
                                            if role_id.get() == guild_id_u64
                                    )
                            })
                        })
                        .map(|cp| cp.id)
                        .collect()
                })
                .unwrap_or_default(),
            _ => HashSet::new(),
        }
    };

    let views = MODULES
        .iter()
        .map(|m| {
            let known: Vec<&Id<CommandMarker>> =
                m.commands.iter().filter_map(|c| name_to_id.get(*c)).collect();
            let enabled =
                known.is_empty() || known.iter().any(|id| !denied.contains(*id));
            ModuleView {
                id: m.id.to_string(),
                label: m.label.to_string(),
                description: m.description.to_string(),
                commands: m.commands.iter().map(|c| (*c).to_string()).collect(),
                enabled,
            }
        })
        .collect();

    Ok(views)
}

#[server]
pub async fn set_module_enabled(
    guild: String,
    module_id: String,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use std::sync::Arc;

    use reqwest::Client;
    use twilight_model::application::command::permissions::{
        CommandPermission,
        CommandPermissionType,
    };
    use zayden_app::state::AppState;

    use crate::app::DiscordBotToken;
    use crate::server::auth::{guild_admin_context, server_err};

    #[derive(serde::Serialize)]
    struct PermissionsBody {
        permissions: Vec<CommandPermission>,
    }

    let (guild_id_i64, _user_id, access_token) = guild_admin_context(&guild).await?;
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(module) = MODULES.iter().find(|m| m.id == module_id) else {
        return Err(ServerFnError::ServerError("unknown module".to_string()));
    };

    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let bot_token =
        use_context::<DiscordBotToken>().map(|t| t.0).unwrap_or_default();
    let app_id = app.zayden_id;

    let name_to_id = fetch_command_ids(&http, app_id, &bot_token).await;

    let body = if enabled {
        PermissionsBody { permissions: Vec::new() }
    } else {
        PermissionsBody {
            permissions: vec![CommandPermission {
                id: CommandPermissionType::Role(Id::new(guild_id_u64)),
                permission: false,
            }],
        }
    };

    for name in module.commands {
        let Some(cmd_id) = name_to_id.get(*name) else {
            continue;
        };
        let url = format!(
            "https://discord.com/api/v10/applications/{app_id}/guilds/{guild_id_u64}/commands/{cmd_id}/permissions"
        );
        let resp = http
            .put(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await
            .map_err(server_err)?;
        if !resp.status().is_success() {
            let status = resp.status();
            return Err(ServerFnError::ServerError(format!(
                "Discord rejected the permission update for /{name} ({status})"
            )));
        }
    }

    Ok(())
}
