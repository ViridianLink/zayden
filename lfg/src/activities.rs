use core::fmt;

pub const ACTIVITIES: [Activity; 41] = [
    //region: Raids
    Activity::new("Desert Perpetual", ActivityCategory::Raid, 6),
    Activity::new("Salvation's Edge", ActivityCategory::Raid, 6),
    Activity::new("Crota's End", ActivityCategory::Raid, 6),
    Activity::new("Root of Nightmares", ActivityCategory::Raid, 6),
    Activity::new("King's Fall", ActivityCategory::Raid, 6),
    Activity::new("Vow of the Disciple", ActivityCategory::Raid, 6),
    Activity::new("Vault of Glass", ActivityCategory::Raid, 6),
    Activity::new("Deep Stone Crypt", ActivityCategory::Raid, 6),
    Activity::new("Garden of Salvation", ActivityCategory::Raid, 6),
    Activity::new("Last Wish", ActivityCategory::Raid, 6),
    Activity::new("Wrath of the Machine", ActivityCategory::Raid, 6),
    Activity::new("Any Raid", ActivityCategory::Raid, 6),
    //endregion
    //region: Dungeons
    Activity::new("Sundered Doctrine", ActivityCategory::Dungeon, 3),
    Activity::new("Vesper's Host", ActivityCategory::Dungeon, 3),
    Activity::new("Warlord's Ruin", ActivityCategory::Dungeon, 3),
    Activity::new("Ghosts of the Deep", ActivityCategory::Dungeon, 3),
    Activity::new("Spire of the Watcher", ActivityCategory::Dungeon, 3),
    Activity::new("Duality", ActivityCategory::Dungeon, 3),
    Activity::new("Grasp of Avarice", ActivityCategory::Dungeon, 3),
    Activity::new("Prophecy", ActivityCategory::Dungeon, 3),
    Activity::new("Pit of Heresy", ActivityCategory::Dungeon, 3),
    Activity::new("Shattered Throne", ActivityCategory::Dungeon, 3),
    Activity::new("Any Dungeon", ActivityCategory::Dungeon, 3),
    //endregion
    Activity::new("Kell's Fall", ActivityCategory::ExoticMission, 3),
    Activity::new("Duel Destiny", ActivityCategory::ExoticMission, 2),
    Activity::new("The Whisper", ActivityCategory::ExoticMission, 3),
    Activity::new("Zero Hour", ActivityCategory::ExoticMission, 3),
    Activity::new("Harbinger", ActivityCategory::ExoticMission, 3),
    Activity::new("Presage", ActivityCategory::ExoticMission, 3),
    Activity::new("Vox Obscura", ActivityCategory::ExoticMission, 3),
    Activity::new(
        "Operation: Seraph's Shield",
        ActivityCategory::ExoticMission,
        3,
    ),
    Activity::new("Node.Ovrd.Avalon", ActivityCategory::ExoticMission, 3),
    Activity::new("Starcrossed", ActivityCategory::ExoticMission, 3),
    Activity::new("Vanguard Ops", ActivityCategory::Vanguard, 3),
    Activity::new("Nightfall", ActivityCategory::Vanguard, 3),
    Activity::new("Grandmaster", ActivityCategory::Vanguard, 3),
    Activity::new("Onslaught", ActivityCategory::Vanguard, 3),
    Activity::new("Crucible", ActivityCategory::Pvp, 6),
    Activity::new("Competitive", ActivityCategory::Pvp, 6),
    Activity::new("Iron Banner", ActivityCategory::Pvp, 6),
    Activity::new("Trials of Osiris", ActivityCategory::Pvp, 3),
];

pub struct Activity {
    pub name: &'static str,
    pub category: ActivityCategory,
    pub fireteam_size: i16,
}

impl Activity {
    const fn new(name: &'static str, category: ActivityCategory, fireteam_size: i16) -> Self {
        Self {
            name,
            category,
            fireteam_size,
        }
    }
}

pub enum ActivityCategory {
    Raid,
    Dungeon,
    ExoticMission,
    Vanguard,
    Pvp,
}

impl fmt::Display for ActivityCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raid => write!(f, "Raid"),
            Self::Dungeon => write!(f, "Dungeon"),
            Self::ExoticMission => write!(f, "Exotic Mission"),
            Self::Vanguard => write!(f, "Vanguard"),
            Self::Pvp => write!(f, "PvP"),
        }
    }
}
