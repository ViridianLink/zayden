mod store;

use std::borrow::Cow;
use std::collections::BTreeMap;

use jiff::Timestamp;
pub use store::{ModuleStates, ModuleStore};

use crate::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backing {
    Commands,
    Settings,
    Derived,
}

#[derive(Debug)]
pub struct ModuleDef {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub backing: Backing,
    pub introduced: Option<Timestamp>,
}

impl ModuleDef {
    #[must_use]
    pub fn default_enabled(&self, bot_joined_at: Timestamp) -> bool {
        self.introduced.is_none_or(|introduced| bot_joined_at >= introduced)
    }
}

pub const MODULES: &[ModuleDef] = &[
    ModuleDef {
        id: "music",
        label: "Music",
        description: "Voice playback, queue, and 24/7 (Pro).",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "palworld",
        label: "Palworld",
        description: "Save parsing, breeding solver, and world sync.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "patreon",
        label: "Patreon",
        description: "Announce a connected Patreon campaign's new posts.",
        backing: Backing::Derived,
        introduced: None,
    },
    ModuleDef {
        id: "youtube",
        label: "YouTube",
        description: "Announce a connected YouTube channel's new uploads.",
        backing: Backing::Derived,
        introduced: None,
    },
    ModuleDef {
        id: "marathon",
        label: "Marathon",
        description: "Marathon wiki lookups and news.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "gambling",
        label: "Gambling & Economy",
        description: "Currency games, shop, and leaderboards.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "family",
        label: "Family",
        description: "Marriage, adoption, and family tree commands.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "ticket",
        label: "Tickets & Support",
        description: "Support tickets and FAQ panels.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "honeypot",
        label: "Honeypot",
        description: "Decoy channel that soft-bans spam bots on sight.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "greetings",
        label: "Greetings",
        description: "Good morning / good night images and messages.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "moderation",
        label: "Moderation",
        description: "Infractions, user logs, and server rules.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "hosting",
        label: "Game Server Hosting",
        description: "Rent and manage hosted game servers.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "jellyfin",
        label: "Jellyfin",
        description: "Jellyfin accounts and watch parties.",
        backing: Backing::Commands,
        introduced: None,
    },
    ModuleDef {
        id: "ai",
        label: "AI Chat",
        description: "Zayden replies in character when he's mentioned.",
        backing: Backing::Settings,
        introduced: None,
    },
    ModuleDef {
        id: "misc",
        label: "Misc",
        description: "Miscellaneous utility commands.",
        backing: Backing::Commands,
        introduced: None,
    },
];

#[must_use]
pub fn find(id: &str) -> Option<&'static ModuleDef> {
    MODULES.iter().find(|m| m.id == id)
}

pub fn command_modules() -> impl Iterator<Item = &'static ModuleDef> {
    MODULES.iter().filter(|m| m.backing == Backing::Commands)
}

pub fn validate(
    commands_by_module: &BTreeMap<&str, Vec<Cow<'_, str>>>,
) -> Result<(), AppError> {
    for (module, commands) in commands_by_module {
        if !command_modules().any(|m| m.id == *module) {
            return Err(AppError::UnknownModule {
                module: (*module).to_owned(),
                commands: commands.iter().map(ToString::to_string).collect(),
            });
        }
    }

    if let Some(module) =
        command_modules().find(|m| !commands_by_module.contains_key(m.id))
    {
        return Err(AppError::ModuleWithoutCommands(module.id));
    }

    Ok(())
}
