use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// <https://github.com/mlg404/palworld-paldex-api>
    Paldex,
    /// <https://paldb.cc>
    PalDb,
    /// <https://palworld.gg>
    PalworldGg,
    /// <https://palworld.fandom.com>
    Fandom,
}

impl SourceId {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paldex => "Paldex",
            Self::PalDb => "PalDB",
            Self::PalworldGg => "Palworld.gg",
            Self::Fandom => "Fandom",
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
    Breeding,
    Suitability,
    Drops,
    Items,
    Passives,
    Lore,
}

impl Category {
    #[must_use]
    pub const fn precedence(self) -> &'static [SourceId] {
        match self {
            Self::Stats
            | Self::Breeding
            | Self::Suitability
            | Self::Items
            | Self::Passives => {
                &[SourceId::Paldex, SourceId::PalDb, SourceId::PalworldGg]
            },
            Self::Drops => {
                &[SourceId::PalDb, SourceId::Paldex, SourceId::PalworldGg]
            },
            Self::Lore => &[SourceId::Fandom, SourceId::Paldex, SourceId::PalDb],
        }
    }

    #[must_use]
    pub fn rank(self, source: SourceId) -> usize {
        let prec = self.precedence();
        prec.iter().position(|&s| s == source).unwrap_or(prec.len())
    }
}
