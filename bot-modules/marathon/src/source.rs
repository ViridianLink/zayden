use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// <https://marathondb.gg>
    MarathonDb,
    /// <https://marathonthegame.fandom.com>
    Fandom,
    /// <https://mobalytics.gg/marathon>
    Mobalytics,
    /// <https://mapgenie.io/marathon>
    MapGenie,
    /// <https://tauceti.gg>
    TauCeti,
    /// <https://marathon-guide.com>
    MarathonGuide,
    /// <https://cyberacme.org>
    CyberAcme,
    /// <https://marathonmeta.gg>
    MarathonMeta,
    /// <https://metaforge.app>
    MetaForge,
}

impl SourceId {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MarathonDb => "MarathonDB",
            Self::Fandom => "Fandom",
            Self::Mobalytics => "Mobalytics",
            Self::MapGenie => "MapGenie",
            Self::TauCeti => "TauCeti",
            Self::MarathonGuide => "Marathon Guide",
            Self::CyberAcme => "CyberAcme",
            Self::MarathonMeta => "Marathon Meta",
            Self::MetaForge => "MetaForge",
        }
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Stats,
    Lore,
    Attachments,
    Faction,
    Map,
    Meta,
}

impl Category {
    #[must_use]
    pub const fn precedence(self) -> &'static [SourceId] {
        match self {
            Self::Stats => &[
                SourceId::MarathonDb,
                SourceId::MarathonGuide,
                SourceId::TauCeti,
                SourceId::CyberAcme,
                SourceId::MarathonMeta,
                SourceId::Mobalytics,
            ],
            Self::Lore => {
                &[SourceId::Fandom, SourceId::TauCeti, SourceId::MarathonDb]
            },
            Self::Attachments => &[
                SourceId::MarathonDb,
                SourceId::TauCeti,
                SourceId::MarathonGuide,
                SourceId::Mobalytics,
            ],
            Self::Faction => &[
                SourceId::CyberAcme,
                SourceId::MarathonGuide,
                SourceId::MarathonDb,
                SourceId::TauCeti,
                SourceId::Mobalytics,
            ],
            Self::Map => &[
                SourceId::MapGenie,
                SourceId::MetaForge,
                SourceId::TauCeti,
                SourceId::MarathonDb,
            ],
            Self::Meta => &[SourceId::MarathonMeta, SourceId::Mobalytics],
        }
    }

    #[must_use]
    pub fn rank(self, source: SourceId) -> usize {
        let prec = self.precedence();
        prec.iter().position(|&s| s == source).unwrap_or(prec.len())
    }
}
