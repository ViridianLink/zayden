#[cfg(feature = "ssr")]
use std::collections::HashMap;

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
    http: &twilight_http::Client,
    app_id: u64,
) -> HashMap<String, Id<CommandMarker>> {
    let Ok(resp) = http.interaction(Id::new(app_id)).global_commands().await else {
        return HashMap::new();
    };
    resp.models()
        .await
        .map(|cmds| {
            cmds.into_iter().filter_map(|c| c.id.map(|id| (c.name, id))).collect()
        })
        .unwrap_or_default()
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    use std::collections::HashSet;
    use std::sync::Arc;

    use zayden_app::state::AppState;

    use crate::server::auth::{bearer_client, guild_admin_context};

    let (guild_id_i64, _user_id, access_token) = guild_admin_context(&guild).await?;
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(http) = use_context::<Arc<twilight_http::Client>>() else {
        return Err(ServerFnError::ServerError(
            "missing Discord client".to_string(),
        ));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let app_id = app.zayden_id;

    let name_to_id = fetch_command_ids(&http, app_id).await;

    let denied: HashSet<Id<CommandMarker>> = {
        use twilight_model::application::command::permissions::CommandPermissionType;

        let resp = bearer_client(&access_token)
            .interaction(Id::new(app_id))
            .guild_command_permissions(Id::new(guild_id_u64))
            .await;
        match resp {
            Ok(r) => r
                .models()
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

    use twilight_model::application::command::permissions::{
        CommandPermission,
        CommandPermissionType,
    };
    use zayden_app::state::AppState;

    use crate::server::auth::{bearer_client, guild_admin_context};

    let (guild_id_i64, _user_id, access_token) = guild_admin_context(&guild).await?;
    let guild_id_u64 = guild_id_i64.cast_unsigned();

    let Some(module) = MODULES.iter().find(|m| m.id == module_id) else {
        return Err(ServerFnError::ServerError("unknown module".to_string()));
    };

    let Some(http) = use_context::<Arc<twilight_http::Client>>() else {
        return Err(ServerFnError::ServerError(
            "missing Discord client".to_string(),
        ));
    };
    let Some(app) = use_context::<Arc<AppState>>() else {
        return Err(ServerFnError::ServerError("missing app state".to_string()));
    };
    let app_id = app.zayden_id;

    let name_to_id = fetch_command_ids(&http, app_id).await;

    let permissions: Vec<CommandPermission> = if enabled {
        Vec::new()
    } else {
        vec![CommandPermission {
            id: CommandPermissionType::Role(Id::new(guild_id_u64)),
            permission: false,
        }]
    };

    let bearer = bearer_client(&access_token);
    let interaction = bearer.interaction(Id::new(app_id));
    for name in module.commands {
        let Some(cmd_id) = name_to_id.get(*name) else {
            continue;
        };
        if let Err(e) = interaction
            .update_command_permissions(Id::new(guild_id_u64), *cmd_id, &permissions)
            .await
        {
            return Err(ServerFnError::ServerError(format!(
                "Discord rejected the permission update for /{name}: {e}"
            )));
        }
    }

    Ok(())
}
