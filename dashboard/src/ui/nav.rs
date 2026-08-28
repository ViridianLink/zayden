#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dest {
    Section { slug: &'static str, lead: &'static str },
    Page(&'static str),
}

#[derive(PartialEq, Eq)]
pub(crate) struct ModuleNav {
    pub(crate) label: &'static str,
    pub(crate) dest: Dest,
    pub(crate) module_id: Option<&'static str>,
}

impl ModuleNav {
    pub(crate) fn href(&self, guild_id: &str) -> String {
        match self.dest {
            Dest::Section { slug, .. } => {
                format!("/guild/{guild_id}/settings/{slug}")
            },
            Dest::Page(slug) => format!("/guild/{guild_id}/{slug}"),
        }
    }

    pub(crate) const fn slug(&self) -> Option<&'static str> {
        match self.dest {
            Dest::Section { slug, .. } => Some(slug),
            Dest::Page(_) => None,
        }
    }

    pub(crate) const fn lead(&self) -> &'static str {
        match self.dest {
            Dest::Section { lead, .. } => lead,
            Dest::Page(_) => "",
        }
    }
}

pub(crate) const GENERAL: ModuleNav = ModuleNav {
    label: "General",
    dest: Dest::Section {
        slug: "general",
        lead: "Server-wide channels and roles the rest of Zayden points at.",
    },
    module_id: None,
};

pub(crate) const MODULES: &[ModuleNav] = &[
    GENERAL,
    ModuleNav {
        label: "AI Chat",
        dest: Dest::Section {
            slug: "ai",
            lead: "Whether Zayden answers when mentioned, and where.",
        },
        module_id: Some("ai"),
    },
    ModuleNav {
        label: "Family",
        dest: Dest::Section {
            slug: "family",
            lead: "Limits for the family and relationship commands.",
        },
        module_id: Some("family"),
    },
    ModuleNav {
        label: "Greetings",
        dest: Dest::Page("greetings"),
        module_id: Some("greetings"),
    },
    ModuleNav {
        label: "Honeypot",
        dest: Dest::Section {
            slug: "honeypot",
            lead: "The spam trap: a bait channel that bans whoever posts in it.",
        },
        module_id: Some("honeypot"),
    },
    ModuleNav { label: "Levels", dest: Dest::Page("levels"), module_id: None },
    ModuleNav {
        label: "LFG",
        dest: Dest::Section {
            slug: "lfg",
            lead: "Where looking-for-group posts go and who they ping.",
        },
        module_id: None,
    },
    ModuleNav {
        label: "Music",
        dest: Dest::Section {
            slug: "music",
            lead: "Playback permissions and now-playing announcements.",
        },
        module_id: Some("music"),
    },
    ModuleNav {
        label: "Reaction Roles",
        dest: Dest::Page("reaction-roles"),
        module_id: None,
    },
    ModuleNav {
        label: "Support",
        dest: Dest::Section {
            slug: "support",
            lead: "Tickets, FAQ and suggestions - where they live and who gets pinged.",
        },
        module_id: Some("ticket"),
    },
    ModuleNav {
        label: "Temp Voice",
        dest: Dest::Section {
            slug: "temp-voice",
            lead: "On-demand voice channels created from a join-to-create channel.",
        },
        module_id: None,
    },
];

pub(crate) fn section(slug: &str) -> &'static ModuleNav {
    MODULES.iter().find(|m| m.slug() == Some(slug)).unwrap_or(&GENERAL)
}

pub(crate) fn for_module(module_id: &str) -> Option<&'static ModuleNav> {
    MODULES.iter().find(|m| m.module_id == Some(module_id))
}
