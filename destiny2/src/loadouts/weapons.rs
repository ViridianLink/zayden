use std::fmt::Display;

use serenity::all::{
    CreateSectionComponent, CreateTextDisplay, CreateThumbnail, CreateUnfurledMediaItem, EmojiId,
};

pub const DEVILS_RUIN: Weapon = Weapon {
    name: "Devil's Ruin",
    affinity: Affinity::Solar,
    archtype: "Exotic Sidearm",
    perks: [Perk::Pyrogenesis, Perk::CloseTheGap],
};

pub const MINT_RETROGRADE: Weapon = Weapon {
    name: "Mint Retrograde",
    affinity: Affinity::Strand,
    archtype: "Legendary Pulse Rifle",
    perks: [Perk::RewindRounds, Perk::Slice],
};

pub const PERFECT_PARADOX: Weapon = Weapon {
    name: "Perfect Paradox",
    affinity: Affinity::Kinetic,
    archtype: "Legendary Shotgun",
    perks: [Perk::ThreatDetector, Perk::OneTwoPunch],
};

pub const THIRD_ITERATION: Weapon = Weapon {
    name: "Third Iteration",
    affinity: Affinity::Void,
    archtype: "Exotic Scout Rifle",
    perks: [Perk::AmalgamationRounds, Perk::TriPlanarMassDriver],
};

pub const SUNSHOT: Weapon = Weapon {
    name: "Sunshot",
    affinity: Affinity::Solar,
    archtype: "Exotic Hand Cannon",
    perks: [Perk::SunBlast, Perk::Sunburn],
};

#[derive(Clone, Copy)]
pub struct Weapon<'a> {
    pub name: &'a str,
    pub affinity: Affinity,
    pub archtype: &'a str,
    pub perks: [Perk; 2],
}

impl<'a> From<Weapon<'a>> for CreateSectionComponent<'a> {
    fn from(value: Weapon<'a>) -> Self {
        CreateSectionComponent::TextDisplay(value.into())
    }
}

impl<'a> From<Weapon<'a>> for CreateTextDisplay<'a> {
    fn from(value: Weapon<'a>) -> Self {
        let perks = value.perks.map(|p| format!(" {p}")).join(" ");

        CreateTextDisplay::new(format!(
            "**{}**\n<:{}:{}> {}\n#{perks}",
            value.name,
            value.affinity,
            EmojiId::from(value.affinity),
            value.archtype,
        ))
    }
}

impl<'a> From<Weapon<'a>> for CreateThumbnail<'a> {
    fn from(value: Weapon<'a>) -> Self {
        CreateThumbnail::new(value.into())
    }
}

impl<'a> From<Weapon<'a>> for CreateUnfurledMediaItem<'a> {
    fn from(value: Weapon) -> Self {
        let url = match value.name {
            "Perfect Paradox" => {
                "https://www.bungie.net/common/destiny2_content/icons/a1eda8ee294310235e24700ae40e7ec2.jpg"
            }
            "Devil's Ruin" => {
                "https://www.bungie.net/common/destiny2_content/icons/32e03608e5f0c25002a2e7abcbcf7ac7.jpg"
            }
            "Third Iteration" => {
                "https://www.bungie.net/common/destiny2_content/icons/58975dd6281e30bac63e9e6b3c868865.jpg"
            }
            "Mint Retrograde" => {
                "https://www.bungie.net/common/destiny2_content/icons/fbf7032cc563c82757be6a7fe5e88713.jpg"
            }
            "Sunshot" => {
                "https://www.bungie.net/common/destiny2_content/icons/b84b4aecd0b0b36b9b9bf247b290ba08.jpg"
            }
            name => unimplemented!("Image URL for '{name}' not implemented"),
        };

        CreateUnfurledMediaItem::new(url)
    }
}

#[derive(Clone, Copy)]
pub enum Affinity {
    Kinetic,
    Arc,
    Void,
    Solar,
    Stasis,
    Strand,
}

impl Display for Affinity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Affinity::Kinetic => "kinetic",
            Affinity::Arc => "arc",
            Affinity::Void => "void",
            Affinity::Solar => "solar",
            Affinity::Stasis => "stasis",
            Affinity::Strand => "strand",
        };

        write!(f, "{name}")
    }
}

impl From<Affinity> for EmojiId {
    fn from(value: Affinity) -> Self {
        match value {
            Affinity::Kinetic => EmojiId::new(1395891226393317456),
            Affinity::Arc => todo!(),
            Affinity::Void => EmojiId::new(1396107597123293254),
            Affinity::Solar => EmojiId::new(1395737098220212345),
            Affinity::Stasis => todo!(),
            Affinity::Strand => EmojiId::new(1399543110157209765),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Perk {
    Pyrogenesis,
    CloseTheGap,
    ThreatDetector,
    OneTwoPunch,
    TriPlanarMassDriver,
    AmalgamationRounds,
    RewindRounds,
    Slice,
    SunBlast,
    Sunburn,
}

impl Display for Perk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Perk::Pyrogenesis => "pyrogenesis",
            Perk::CloseTheGap => "close_the_gap",
            Perk::ThreatDetector => "threat_detector",
            Perk::OneTwoPunch => "one_two_punch",
            Perk::TriPlanarMassDriver => "tri_planar_mass_driver",
            Perk::AmalgamationRounds => "amalgamation_rounds",
            Perk::RewindRounds => "rewind_rounds",
            Perk::Slice => "slice",
            Perk::SunBlast => "sun_blast",
            Perk::Sunburn => "sunburn",
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Perk> for EmojiId {
    fn from(value: Perk) -> Self {
        match value {
            Perk::Pyrogenesis => EmojiId::new(1395892081246998568),
            Perk::CloseTheGap => EmojiId::new(1395892132887134279),
            Perk::ThreatDetector => EmojiId::new(1395892185945215018),
            Perk::OneTwoPunch => EmojiId::new(1395892237086490655),
            Perk::TriPlanarMassDriver => EmojiId::new(1396093613330665493),
            Perk::AmalgamationRounds => EmojiId::new(1396093652006211706),
            Perk::RewindRounds => EmojiId::new(1399522910091214878),
            Perk::Slice => EmojiId::new(1399522938805424341),
            Perk::SunBlast => EmojiId::new(1399522958191493230),
            Perk::Sunburn => EmojiId::new(1399522978131087511),
        }
    }
}
