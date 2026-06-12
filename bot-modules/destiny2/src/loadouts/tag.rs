use std::fmt;
use std::fmt::{Display, Formatter};

use serenity::all::{ButtonStyle, CreateButton};

#[derive(Clone, Copy)]
pub enum Tag {
    EasyToPlay,
    BossDamage,
    AdClear,
    HighSurvivability,
    Support,
    AntiChampion,
    CasualPvP,
    CompetitivePvp,
    Raids,
    Dungeons,
    MasterContent,
    GrandmasterNightfall,
    Solo,
    SuperFocused,
    AbilityFocused,
    WeaponFocused,
    HighDamage,
    EndGame,
    CrowdControl,
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::EasyToPlay => "Easy To Play",
            Self::BossDamage => "Boss Damage",
            Self::AdClear => "Ad Clear",
            Self::HighSurvivability => "High Survivability",
            Self::Support => "Support",
            Self::AntiChampion => "Anti-Champion",
            Self::CasualPvP => "Casual PvP",
            Self::CompetitivePvp => "Competitive PvP",
            Self::Raids => "Raids",
            Self::Dungeons => "Dungeons",
            Self::MasterContent => "Master Content",
            Self::GrandmasterNightfall => "Grandmaster Nightfall",
            Self::Solo => "Solo",
            Self::SuperFocused => "Super Focused",
            Self::AbilityFocused => "Ability Focused",
            Self::WeaponFocused => "Weapon Focused",
            Self::HighDamage => "High Damage",
            Self::EndGame => "End Game",
            Self::CrowdControl => "Crowd Control",
        };

        write!(f, "{name}")
    }
}

impl From<Tag> for CreateButton<'_> {
    fn from(value: Tag) -> Self {
        CreateButton::new(format!("{value}"))
            .label(format!("{value}"))
            .style(ButtonStyle::Secondary)
    }
}
