use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{GuildAccess, app_state, server_err},
    crate::server::command_permissions::{
        GuildContext,
        everyone_denied,
        fetch_command_ids,
        guild_context,
        guild_permissions,
        store,
        with_everyone_denied,
    },
    crate::server::patreon::fetch_patreon_status,
    crate::server::supersede,
    std::collections::{HashMap, HashSet},
    twilight_model::application::command::permissions::CommandPermission,
    twilight_model::id::Id,
    twilight_model::id::marker::{CommandMarker, GuildMarker},
};

use crate::dto::ModuleView;

#[cfg(feature = "ssr")]
enum Backing {
    Commands(&'static [&'static str]),
    Settings,
    Derived,
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
        id: "patreon",
        label: "Patreon",
        description: "Announce a connected Patreon campaign's new posts.",
        backing: Backing::Derived,
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
        backing: Backing::Commands(&["family"]),
    },
    ModuleDef {
        id: "ticket",
        label: "Tickets & Support",
        description: "Support tickets and FAQ panels.",
        backing: Backing::Commands(&["ticket"]),
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
            Backing::Settings | Backing::Derived => &[],
        }
    }

    const fn locked_for(&self, access: GuildAccess) -> Option<&'static str> {
        match self.backing {
            Backing::Commands(_) if !access.can_write_command_permissions() => Some(
                "Read-only: Discord only lets a member with Manage Server \
                     change which commands are enabled.",
            ),
            Backing::Derived => Some(
                "This module is switched on from its own settings page \u{2014} \
                 use Configure below.",
            ),
            Backing::Commands(_) | Backing::Settings => None,
        }
    }

    fn view(
        &self,
        commands: Option<&CommandState>,
        settings_flags: &HashMap<&'static str, bool>,
        access: GuildAccess,
    ) -> ModuleView {
        let enabled = match self.backing {
            Backing::Commands(names) => {
                commands.and_then(|state| state.enabled(names))
            },
            Backing::Settings | Backing::Derived => {
                settings_flags.get(self.id).copied()
            },
        };

        ModuleView {
            id: self.id.to_string(),
            label: self.label.to_string(),
            description: self.description.to_string(),
            commands: self.commands().iter().map(|c| (*c).to_string()).collect(),
            enabled,
            locked: self.locked_for(access).map(str::to_owned),
        }
    }
}

#[cfg(feature = "ssr")]
pub struct CommandState {
    name_to_id: HashMap<String, Id<CommandMarker>>,
    denied: HashSet<Id<CommandMarker>>,
}

#[cfg(feature = "ssr")]
impl CommandState {
    #[must_use]
    pub fn new(
        guild_id: Id<GuildMarker>,
        name_to_id: HashMap<String, Id<CommandMarker>>,
        permissions: &HashMap<Id<CommandMarker>, Vec<CommandPermission>>,
    ) -> Self {
        Self { name_to_id, denied: denied_commands(guild_id, permissions) }
    }

    #[must_use]
    pub fn enabled(&self, names: &[&str]) -> Option<bool> {
        let known: Vec<_> =
            names.iter().filter_map(|c| self.name_to_id.get(*c)).collect();

        if known.is_empty() {
            return None;
        }

        Some(known.iter().any(|id| !self.denied.contains(*id)))
    }
}

#[cfg(feature = "ssr")]
fn denied_commands(
    guild_id: Id<GuildMarker>,
    permissions: &HashMap<Id<CommandMarker>, Vec<CommandPermission>>,
) -> HashSet<Id<CommandMarker>> {
    permissions
        .iter()
        .filter(|(_id, perms)| everyone_denied(guild_id, perms))
        .map(|(id, _perms)| *id)
        .collect()
}

#[cfg(feature = "ssr")]
async fn settings_flags(
    guild_id: i64,
) -> Result<HashMap<&'static str, bool>, ServerFnError> {
    let app = app_state()?;

    let (ai, patreon) = tokio::try_join!(
        async { app.settings.ai.get(guild_id).await.map_err(server_err) },
        fetch_patreon_status(&app, guild_id),
    )?;

    // Announcements only fire on a live connection with somewhere to post.
    let patreon_on =
        patreon.connected && !patreon.disabled && patreon.channel_id.is_some();

    Ok(HashMap::from([("ai", ai.enabled), ("patreon", patreon_on)]))
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    let ctx = guild_context(&guild).await?;

    let (name_to_id, permissions, flags) = tokio::join!(
        fetch_command_ids(&ctx),
        guild_permissions(&ctx),
        settings_flags(ctx.guild_id.get().cast_signed()),
    );
    let flags = flags?;

    let commands = match (name_to_id, permissions) {
        (Ok(name_to_id), Ok(permissions)) => {
            Some(CommandState::new(ctx.guild_id, name_to_id, &permissions))
        },
        (Err(e), _) | (_, Err(e)) => {
            tracing::warn!(
                error = ?e,
                "failed to read command state from Discord; reporting modules as unknown"
            );
            None
        },
    };

    Ok(MODULES
        .iter()
        .map(|m| m.view(commands.as_ref(), &flags, ctx.access))
        .collect())
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

#[cfg(feature = "ssr")]
async fn set_commands_enabled(
    ctx: &GuildContext,
    claim: &supersede::Claim,
    names: &[&str],
    enabled: bool,
) -> Result<(), ServerFnError> {
    let (name_to_id, permissions) =
        tokio::join!(fetch_command_ids(ctx), guild_permissions(ctx));
    let name_to_id = name_to_id?;
    let mut permissions = permissions?;

    let mut resolved = Vec::with_capacity(names.len());
    let mut missing = Vec::new();

    for name in names {
        match name_to_id.get(*name) {
            Some(id) => resolved.push((*name, *id)),
            None => missing.push(format!("/{name}")),
        }
    }

    if !missing.is_empty() {
        return Err(ServerFnError::ServerError(format!(
            "Not registered for this server yet: {}",
            missing.join(", ")
        )));
    }

    for (name, cmd_id) in resolved {
        if claim.superseded() {
            return Ok(());
        }

        let current = permissions.remove(&cmd_id).unwrap_or_default();

        // Skip the ones that already read the way the toggle wants them.
        if everyone_denied(ctx.guild_id, &current) != enabled {
            continue;
        }

        let updated = with_everyone_denied(ctx.guild_id, &current, !enabled);

        store(ctx, cmd_id, name, &updated).await?;
    }

    Ok(())
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

    let claim = supersede::claim(ctx.guild_id, module.id);
    let _turn = claim.wait_for_turn().await;

    if claim.superseded() {
        return Ok(());
    }

    match module.backing {
        Backing::Settings => {
            set_settings_enabled(
                module.id,
                ctx.guild_id.get().cast_signed(),
                enabled,
            )
            .await
        },
        Backing::Commands(names) => {
            set_commands_enabled(&ctx, &claim, names, enabled).await
        },
        Backing::Derived => Err(ServerFnError::ServerError(format!(
            "{} is switched on from its own settings page, not from this toggle.",
            module.label
        ))),
    }
}
