use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::bearer_client,
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
        id: "honeypot",
        label: "Honeypot",
        description: "Decoy channel that soft-bans spam bots on sight.",
        commands: &["honeypot"],
    },
    ModuleDef {
        id: "greetings",
        label: "Greetings",
        description: "Good morning / good night images and messages.",
        commands: &["good"],
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

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    let ctx = guild_context(&guild).await?;

    let name_to_id = fetch_command_ids(&ctx).await;
    let denied = denied_commands(&ctx).await;

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
    let name_to_id = fetch_command_ids(&ctx).await;

    for name in module.commands {
        let Some(cmd_id) = name_to_id.get(*name) else {
            continue;
        };

        let current = fetch(&ctx, *cmd_id).await;
        let updated = with_everyone_denied(ctx.guild_id, &current, !enabled);

        store(&ctx, *cmd_id, name, &updated).await?;
    }

    Ok(())
}
