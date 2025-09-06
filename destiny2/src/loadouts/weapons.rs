use std::fmt::Display;

use serenity::all::{
    CreateSectionComponent, CreateTextDisplay, CreateThumbnail, CreateUnfurledMediaItem,
};
use zayden_core::EmojiCache;

pub const DEAD_MESSENGER: Weapon = Weapon {
    name: "Dead Messenger",
    affinity: Affinity::Void,
    archtype: "Exotic Grenade Launcher",
    perks: [Perk::TheFundamentals, Perk::HandLaidStock],
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
    perks: [Perk::FieldPrep, Perk::OneTwoPunch],
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

pub const PHONEUTRIA_FERA: Weapon = Weapon {
    name: "Phoneutria Fera",
    affinity: Affinity::Solar,
    archtype: "Exotic Hand Cannon",
    perks: [Perk::ThreatDetector, Perk::OneTwoPunch],
};

pub const GRAVITON_SPIKE: Weapon = Weapon {
    name: "Graviton Spike",
    affinity: Affinity::Arc,
    archtype: "Exotic Hand Cannon",
    perks: [Perk::TemporalAnomaly, Perk::TemporalManipulation],
};

pub const NAVIGATOR: Weapon = Weapon {
    name: "Navigator",
    affinity: Affinity::Strand,
    archtype: "Exotic Trace Rifle",
    perks: [Perk::WeftCutter, Perk::ProtectiveWeave],
};

pub const IKELOS_SG_V103: Weapon = Weapon {
    name: "IKELOS_SG_v1.0.3",
    affinity: Affinity::Solar,
    archtype: "Legendary Shotgun",
    perks: [Perk::GraveRobber, Perk::OneTwoPunch],
};

pub const MONTE_CARLO: Weapon = Weapon {
    name: "Monte Carlo",
    affinity: Affinity::Kinetic,
    archtype: "Exotic Auto Rifle",
    perks: [Perk::MarkovChain, Perk::MonteCarloMethod],
};

#[derive(Clone, Copy)]
pub struct Weapon<'a> {
    pub name: &'a str,
    pub affinity: Affinity,
    pub archtype: &'a str,
    pub perks: [Perk; 2],
}

impl Weapon<'_> {
    pub fn into_text_display<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> Result<CreateTextDisplay<'a>, String> {
        let perks = self
            .perks
            .into_iter()
            .map(|p| p.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<Result<String, String>>()?;

        let text_display = CreateTextDisplay::new(format!(
            "**{}**\n{} {}\n#{perks}",
            self.name,
            emoji_cache.emoji_str(&self.affinity.to_string())?,
            self.archtype,
        ));

        Ok(text_display)
    }

    pub fn into_section<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> Result<CreateSectionComponent<'a>, String> {
        self.into_text_display(emoji_cache)
            .map(CreateSectionComponent::TextDisplay)
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
            "Phoneutria Fera" => {
                "https://www.bungie.net/common/destiny2_content/icons/f028107777dd4286a213ec2cbd9544f5.jpg"
            }
            "Graviton Spike" => {
                "https://www.bungie.net/common/destiny2_content/icons/ac56ad66eb1ebb8a371f9d3d3c768c5a.jpg"
            }
            "Navigator" => {
                "https://www.bungie.net/common/destiny2_content/icons/8e2b12633d1778a2e502148b0dcafacc.jpg"
            }
            "IKELOS_SG_v1.0.3" => {
                "https://www.bungie.net/common/destiny2_content/icons/e74e5e3e2ee712563245c8ed25b5794c.jpg"
            }
            "Monte Carlo" => {
                "https://www.bungie.net/common/destiny2_content/icons/ad75fa3374e2ce5a549db8d7f672098c.jpg"
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
    FieldPrep,
    TemporalAnomaly,
    TemporalManipulation,
    WeftCutter,
    ProtectiveWeave,
    GraveRobber,
    MarkovChain,
    MonteCarloMethod,
    TheFundamentals,
    HandLaidStock,
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
            Perk::FieldPrep => "field_prep",
            Perk::TemporalAnomaly => "temporal_anomaly",
            Perk::TemporalManipulation => "temporal_manipulation",
            Perk::WeftCutter => "weft_cutter",
            Perk::ProtectiveWeave => "protective_weave",
            Perk::GraveRobber => "grave_robber",
            Perk::MarkovChain => "markov_chain",
            Perk::MonteCarloMethod => "monte_carlo_method",
            Perk::TheFundamentals => "the_fundamentals",
            Perk::HandLaidStock => "hand_laid_stock",
        };

        write!(f, "{name}")
    }
}
