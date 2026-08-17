use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{app_state, bearer_client, server_err},
    crate::server::command_permissions::{
        GuildContext,
        everyone_denied,
        fetch,
        fetch_command_ids,
        guild_context,
        store,
        with_everyone_denied,
    },
    std::collections::{HashMap, HashSet},
    twilight_model::id::Id,
    twilight_model::id::marker::CommandMarker,
};

use crate::dto::ModuleView;

#[cfg(feature = "ssr")]
enum Backing {
    Commands(&'static [&'static str]),
    Settings,
}

#[cfg(feature = "ssr")]
struct ModuleDef {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    backing: Backing,
}

#[cfg(feature = "ssr")]
const MODULES: &[ModuleDef] = &[
    ModuleDef {
        id: "music",
        label: "Music",
        description: "Voice playback, queue, and 24/7 (Pro).",
        backing: Backing::Commands(&["music"]),
    },
    ModuleDef {
        id: "palworld",
        label: "Palworld",
        description: "Save parsing, breeding solver, and world sync.",
        backing: Backing::Commands(&["palworld"]),
    },
    ModuleDef {
        id: "marathon",
        label: "Marathon",
        description: "Marathon wiki lookups and news.",
        backing: Backing::Commands(&["marathon"]),
    },
    ModuleDef {
        id: "gambling",
        label: "Gambling & Economy",
        description: "Currency games, shop, and leaderboards.",
        backing: Backing::Commands(&[
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
        ]),
    },
    ModuleDef {
        id: "family",
        label: "Family",
        description: "Marriage, adoption, and family tree commands.",
        backing: Backing::Commands(&[
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
        ]),
    },
    ModuleDef {
        id: "ticket",
        label: "Tickets & Support",
        description: "Support tickets and FAQ panels.",
        backing: Backing::Commands(&["ticket", "support"]),
    },
    ModuleDef {
        id: "honeypot",
        label: "Honeypot",
        description: "Decoy channel that soft-bans spam bots on sight.",
        backing: Backing::Commands(&["honeypot"]),
    },
    ModuleDef {
        id: "greetings",
        label: "Greetings",
        description: "Good morning / good night images and messages.",
        backing: Backing::Commands(&["good"]),
    },
    ModuleDef {
        id: "ai",
        label: "AI Chat",
        description: "Zayden replies in character when he's mentioned.",
        backing: Backing::Settings,
    },
    ModuleDef {
        id: "misc",
        label: "Misc",
        description: "Miscellaneous utility commands.",
        backing: Backing::Commands(&["random", "custom_msg"]),
    },
];

#[cfg(feature = "ssr")]
impl ModuleDef {
    const fn commands(&self) -> &'static [&'static str] {
        match self.backing {
            Backing::Commands(names) => names,
            Backing::Settings => &[],
        }
    }

    fn view(
        &self,
        name_to_id: &HashMap<String, Id<CommandMarker>>,
        denied: &HashSet<Id<CommandMarker>>,
        settings_flags: &HashMap<&'static str, bool>,
    ) -> ModuleView {
        let enabled = match self.backing {
            Backing::Commands(names) => {
                let known: Vec<_> =
                    names.iter().filter_map(|c| name_to_id.get(*c)).collect();

                known.is_empty() || known.iter().any(|id| !denied.contains(*id))
            },
            Backing::Settings => {
                settings_flags.get(self.id).copied().unwrap_or(false)
            },
        };

        ModuleView {
            id: self.id.to_string(),
            label: self.label.to_string(),
            description: self.description.to_string(),
            commands: self.commands().iter().map(|c| (*c).to_string()).collect(),
            enabled,
        }
    }
}

#[cfg(feature = "ssr")]
async fn denied_commands(ctx: &GuildContext) -> HashSet<Id<CommandMarker>> {
    let resp = bearer_client(&ctx.access_token)
        .interaction(Id::new(ctx.app_id))
        .guild_command_permissions(ctx.guild_id)
        .await;
    let Ok(resp) = resp else {
        return HashSet::new();
    };

    resp.models()
        .await
        .map(|list| {
            list.into_iter()
                .filter(|cp| everyone_denied(ctx.guild_id, &cp.permissions))
                .map(|cp| cp.id)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(feature = "ssr")]
async fn settings_flags(
    guild_id: i64,
) -> Result<HashMap<&'static str, bool>, ServerFnError> {
    let ai = app_state()?.settings.ai.get(guild_id).await.map_err(server_err)?;

    Ok(HashMap::from([("ai", ai.enabled)]))
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    let ctx = guild_context(&guild).await?;

    let name_to_id = fetch_command_ids(&ctx).await;
    let denied = denied_commands(&ctx).await;
    let flags = settings_flags(ctx.guild_id.get().cast_signed()).await?;

    Ok(MODULES.iter().map(|m| m.view(&name_to_id, &denied, &flags)).collect())
}

#[cfg(feature = "ssr")]
async fn set_settings_enabled(
    module_id: &str,
    guild_id: i64,
    enabled: bool,
) -> Result<(), ServerFnError> {
    match module_id {
        "ai" => app_state()?
            .settings
            .ai
            .update(guild_id, |row| row.enabled = enabled)
            .await
            .map(|_row| ())
            .map_err(server_err),
        _ => Err(ServerFnError::ServerError(format!(
            "module {module_id} has no settings switch"
        ))),
    }
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

    let names = match module.backing {
        Backing::Settings => {
            return set_settings_enabled(
                module.id,
                ctx.guild_id.get().cast_signed(),
                enabled,
            )
            .await;
        },
        Backing::Commands(names) => names,
    };

    let name_to_id = fetch_command_ids(&ctx).await;

    for name in names {
        let Some(cmd_id) = name_to_id.get(*name) else {
            continue;
        };

        let current = fetch(&ctx, *cmd_id).await;
        let updated = with_everyone_denied(ctx.guild_id, &current, !enabled);

        store(&ctx, *cmd_id, name, &updated).await?;
    }

    Ok(())
}
