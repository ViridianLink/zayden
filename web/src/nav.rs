use std::fmt::{Display, Write as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dest {
    Section { slug: &'static str, path: &'static str, lead: &'static str },
    Page(&'static str),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModuleNav {
    pub label: &'static str,
    pub dest: Dest,
    pub module_id: Option<&'static str>,
}

impl ModuleNav {
    #[must_use]
    pub fn href(&self, guild_id: impl Display) -> String {
        format!("/guild/{}/{}", path_segment(&guild_id.to_string()), self.path())
    }

    #[must_use]
    pub const fn path(&self) -> &'static str {
        match self.dest {
            Dest::Section { path, .. } => path,
            Dest::Page(slug) => slug,
        }
    }

    #[must_use]
    pub const fn slug(&self) -> Option<&'static str> {
        match self.dest {
            Dest::Section { slug, .. } => Some(slug),
            Dest::Page(_) => None,
        }
    }

    #[must_use]
    pub const fn lead(&self) -> &'static str {
        match self.dest {
            Dest::Section { lead, .. } => lead,
            Dest::Page(_) => "",
        }
    }
}

pub const GENERAL: ModuleNav = ModuleNav {
    label: "Server settings",
    dest: Dest::Section {
        slug: "general",
        path: "settings",
        lead: "Server-wide channels and roles the rest of Zayden points at.",
    },
    module_id: None,
};

const GREETINGS: ModuleNav = ModuleNav {
    label: "Greetings",
    dest: Dest::Page("greetings"),
    module_id: Some("greetings"),
};

const LEVELS: ModuleNav =
    ModuleNav { label: "Levels", dest: Dest::Page("levels"), module_id: None };

const REACTION_ROLES: ModuleNav = ModuleNav {
    label: "Reaction roles",
    dest: Dest::Page("reaction-roles"),
    module_id: None,
};

const FAMILY: ModuleNav = ModuleNav {
    label: "Family",
    dest: Dest::Section {
        slug: "family",
        path: "family",
        lead: "Limits for the family and relationship commands.",
    },
    module_id: Some("family"),
};

const SUPPORT: ModuleNav = ModuleNav {
    label: "Support",
    dest: Dest::Section {
        slug: "support",
        path: "support",
        lead: "Tickets, FAQ and suggestions - where they live and who gets pinged.",
    },
    module_id: Some("ticket"),
};

const HONEYPOT: ModuleNav = ModuleNav {
    label: "Honeypot",
    dest: Dest::Section {
        slug: "honeypot",
        path: "honeypot",
        lead: "The spam trap: a bait channel that bans whoever posts in it.",
    },
    module_id: Some("honeypot"),
};

const MUSIC: ModuleNav = ModuleNav {
    label: "Music",
    dest: Dest::Section {
        slug: "music",
        path: "music",
        lead: "Playback permissions and now-playing announcements.",
    },
    module_id: Some("music"),
};

const TEMP_VOICE: ModuleNav = ModuleNav {
    label: "Temp voice",
    dest: Dest::Section {
        slug: "temp-voice",
        path: "temp-voice",
        lead: "On-demand voice channels created from a join-to-create channel.",
    },
    module_id: None,
};

const LFG: ModuleNav = ModuleNav {
    label: "LFG",
    dest: Dest::Section {
        slug: "lfg",
        path: "lfg",
        lead: "Where looking-for-group posts go and who they ping.",
    },
    module_id: None,
};

const AI_CHAT: ModuleNav = ModuleNav {
    label: "AI Chat",
    dest: Dest::Section {
        slug: "ai",
        path: "ai",
        lead: "Whether Zayden answers when mentioned, and where.",
    },
    module_id: Some("ai"),
};

const PATREON: ModuleNav = ModuleNav {
    label: "Patreon",
    dest: Dest::Section {
        slug: "patreon",
        path: "patreon",
        lead: "Connect a Patreon campaign and choose where its posts are announced.",
    },
    module_id: Some("patreon"),
};

const YOUTUBE: ModuleNav = ModuleNav {
    label: "YouTube",
    dest: Dest::Section {
        slug: "youtube",
        path: "youtube",
        lead: "Connect a YouTube channel and choose where its uploads are announced.",
    },
    module_id: Some("youtube"),
};

#[derive(Debug, PartialEq, Eq)]
pub struct NavGroup {
    pub label: &'static str,
    pub entries: &'static [ModuleNav],
}

pub const GROUPS: &[NavGroup] = &[
    NavGroup {
        label: "Community",
        entries: &[GREETINGS, LEVELS, REACTION_ROLES, FAMILY],
    },
    NavGroup { label: "Support & safety", entries: &[SUPPORT, HONEYPOT] },
    NavGroup { label: "Voice & games", entries: &[MUSIC, TEMP_VOICE, LFG] },
    NavGroup { label: "Integrations", entries: &[AI_CHAT, PATREON, YOUTUBE] },
];

pub const MODULES: &[ModuleNav] = &[
    GENERAL,
    GREETINGS,
    LEVELS,
    REACTION_ROLES,
    FAMILY,
    SUPPORT,
    HONEYPOT,
    MUSIC,
    TEMP_VOICE,
    LFG,
    AI_CHAT,
    PATREON,
    YOUTUBE,
];

#[must_use]
pub fn section(slug: &str) -> &'static ModuleNav {
    MODULES.iter().find(|module| module.slug() == Some(slug)).unwrap_or(&GENERAL)
}

#[must_use]
pub fn settings_href(guild_id: impl Display, slug: &str) -> Option<String> {
    MODULES
        .iter()
        .find(|module| module.slug() == Some(slug))
        .map(|module| module.href(guild_id))
}

#[must_use]
pub fn legacy_href(guild_id: impl Display, slug: &str) -> String {
    MODULES
        .iter()
        .find(|module| match module.dest {
            Dest::Section { slug: section, .. } => section == slug,
            Dest::Page(page) => page == slug,
        })
        .unwrap_or(&GENERAL)
        .href(guild_id)
}

#[must_use]
pub fn settings_entry(path: &str) -> Option<&'static ModuleNav> {
    MODULES.iter().find(|module| {
        matches!(module.dest, Dest::Section { path: own, .. } if own == path)
    })
}

#[must_use]
pub fn for_module(module_id: &str) -> Option<&'static ModuleNav> {
    MODULES.iter().find(|module| module.module_id == Some(module_id))
}

#[must_use]
pub fn path_segment(raw: &str) -> String {
    raw.bytes().fold(String::with_capacity(raw.len()), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}
