use std::fmt;
use std::fmt::{Display, Formatter};

use serenity::all::{
    CreateSectionComponent,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
};
use zayden_core::EmojiCache;

#[derive(Clone, Copy)]
pub enum Weapon {
    RecklessOracle([Perk; 2]),
    LotusEater([Perk; 2]),
    PraxicBlade([Perk; 2]),
    YeartideApex([Perk; 2]),
}

impl Weapon {
    pub fn affinity(self) -> Affinity {
        match self {
            Self::RecklessOracle(_) => Affinity::Void,
            Self::LotusEater(_) => Affinity::Void,
            Self::PraxicBlade(_) => Affinity::Kinetic,
            Self::YeartideApex(_) => Affinity::Solar,
        }
    }

    pub fn archtype(self) -> Archtype {
        match self {
            Self::RecklessOracle(_) => Archtype::AutoRifle,
            Self::LotusEater(_) => Archtype::RocketSidearm,
            Self::PraxicBlade(_) => Archtype::Sword,
            Self::YeartideApex(_) => Archtype::SMG,
        }
    }

    pub fn perks(self) -> [Perk; 2] {
        match self {
            Self::RecklessOracle(perks)
            | Self::LotusEater(perks)
            | Self::PraxicBlade(perks)
            | Self::YeartideApex(perks) => perks,
        }
    }

    pub fn into_text_display<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> Result<CreateTextDisplay<'a>, String> {
        let perks = self
            .perks()
            .into_iter()
            .map(|p| p.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<Result<String, String>>()?;

        let text_display = CreateTextDisplay::new(format!(
            "**{self}**\n{} {}\n#{perks}",
            emoji_cache.emoji_str(&self.affinity().to_string())?,
            self.archtype(),
        ));

        Ok(text_display)
    }

    pub fn into_section<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> Result<CreateSectionComponent<'a>, String> {
        self.into_text_display(emoji_cache).map(CreateSectionComponent::TextDisplay)
    }
}

impl Display for Weapon {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::RecklessOracle(_) => "Reckless Oracle",
            Self::LotusEater(_) => "Lotus Eater",
            Self::PraxicBlade(_) => "Praxic Blade",
            Self::YeartideApex(_) => "Yeartide Apex",
        };

        write!(f, "{s}")
    }
}

impl From<Weapon> for CreateThumbnail<'_> {
    fn from(value: Weapon) -> Self {
        CreateThumbnail::new(value.into())
    }
}

impl From<Weapon> for CreateUnfurledMediaItem<'_> {
    fn from(value: Weapon) -> Self {
        let url = match value {
            // "Perfect Paradox" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/a1eda8ee294310235e24700ae40e7ec2.jpg"
            // },
            // "Devil's Ruin" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/32e03608e5f0c25002a2e7abcbcf7ac7.jpg"
            // },
            // "Third Iteration" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/58975dd6281e30bac63e9e6b3c868865.jpg"
            // },
            // "Mint Retrograde" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/fbf7032cc563c82757be6a7fe5e88713.jpg"
            // },
            // "Sunshot" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/b84b4aecd0b0b36b9b9bf247b290ba08.jpg"
            // },
            // "Phoneutria Fera" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/f028107777dd4286a213ec2cbd9544f5.jpg"
            // },
            // "Graviton Spike" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/ac56ad66eb1ebb8a371f9d3d3c768c5a.jpg"
            // },
            // "Navigator" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/8e2b12633d1778a2e502148b0dcafacc.jpg"
            // },
            // "IKELOS_SG_v1.0.3" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/e74e5e3e2ee712563245c8ed25b5794c.jpg"
            // },
            // "Monte Carlo" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/ad75fa3374e2ce5a549db8d7f672098c.jpg"
            // },
            // "New Malpais" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/d90d7a4fca9a90e3202ed402b87848dd.jpg"
            // },
            // "Dead Messenger" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/c2e44f40d97a0a9eb1af8d25fb8c0f03.jpg"
            // },
            // "Choir of One" => {
            //     "https://www.bungie.net/common/destiny2_content/icons/e285e30c15aa9482df3f1f9c5810fe66.jpg"
            // },
            Weapon::RecklessOracle(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/af58615844b44293f5911ccaae913804.jpg"
            },
            Weapon::LotusEater(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/548f5f1ca7d0bece0ba46d99846e56f7.jpg"
            },
            Weapon::PraxicBlade(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/d63292c9248c5e3ae823605307140199.jpg"
            },
            Weapon::YeartideApex(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/11238fa67ca0881554335d4612eda813.jpg"
            },
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Kinetic => "kinetic",
            Self::Arc => "arc",
            Self::Void => "void",
            Self::Solar => "solar",
            Self::Stasis => "stasis",
            Self::Strand => "strand",
        };

        write!(f, "{name}")
    }
}

pub enum Archtype {
    AutoRifle,
    Bow,
    FusionRifle,
    Glaive,
    BreechGrenadeLauncher,
    GrenadeLauncher,
    HandCannon,
    LinearFusionRifle,
    MachineGun,
    RocketPulseRifle,
    PulseRifle,
    RocketLauncher,
    ScoutRifle,
    Shotgun,
    RocketSidearm,
    Sidearm,
    SMG,
    SniperRifle,
    Sword,
    TraceRifle,
}

impl Display for Archtype {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AutoRifle => "Auto Rifle",
            Self::Bow => "Bow",
            Self::FusionRifle => "Fusion Rifle",
            Self::Glaive => "Glaive",
            Self::BreechGrenadeLauncher => "Breech Grenade Launcher",
            Self::GrenadeLauncher => "Grenade Launcher",
            Self::HandCannon => "Hand Cannon",
            Self::LinearFusionRifle => "Linear Fusion Rifle",
            Self::MachineGun => "Machine Gun",
            Self::RocketPulseRifle => "Rocket Pulse Rifle",
            Self::PulseRifle => "Pulse Rifle",
            Self::RocketLauncher => "Rocket Launcher",
            Self::ScoutRifle => "Scout Rifle",
            Self::Shotgun => "Shotgun",
            Self::RocketSidearm => "Rocket Sidearm",
            Self::Sidearm => "Sidearm",
            Self::SMG => "SMG",
            Self::SniperRifle => "Sniper Rifle",
            Self::Sword => "Sword",
            Self::TraceRifle => "Trace Rifle",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Perk {
    DestabilizingRounds,
    OneForAll,
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
    SuspendingBlast,
    AtomizingRounds,
    CommandFrame,
    FanaticalLance,
    TempestCascade,
    TraitorsVessel,
    RepulsorBrace,
    RangedWeapon,
    CormorantCombo,
    HealClip,
    ChaosReshaped,
}

impl Display for Perk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Pyrogenesis => "pyrogenesis",
            Self::CloseTheGap => "close_the_gap",
            Self::ThreatDetector => "threat_detector",
            Self::OneTwoPunch => "one_two_punch",
            Self::TriPlanarMassDriver => "tri_planar_mass_driver",
            Self::AmalgamationRounds => "amalgamation_rounds",
            Self::RewindRounds => "rewind_rounds",
            Self::Slice => "slice",
            Self::SunBlast => "sun_blast",
            Self::Sunburn => "sunburn",
            Self::FieldPrep => "field_prep",
            Self::TemporalAnomaly => "temporal_anomaly",
            Self::TemporalManipulation => "temporal_manipulation",
            Self::WeftCutter => "weft_cutter",
            Self::ProtectiveWeave => "protective_weave",
            Self::GraveRobber => "grave_robber",
            Self::MarkovChain => "markov_chain",
            Self::MonteCarloMethod => "monte_carlo_method",
            Self::TheFundamentals => "the_fundamentals",
            Self::HandLaidStock => "hand_laid_stock",
            Self::SuspendingBlast => "suspending_blast",
            Self::AtomizingRounds => "atomizing_rounds",
            Self::CommandFrame => "command_frame",
            Self::FanaticalLance => "fanatical_lance",
            Self::TempestCascade => "tempest_cascade",
            Self::TraitorsVessel => "traitors_vessel",
            Self::DestabilizingRounds => "destabilizing_rounds",
            Self::OneForAll => "one_for_all",
            Self::RepulsorBrace => "repulsor_brace",
            Self::RangedWeapon => "ranged_weapon",
            Self::CormorantCombo => "cormorant_combo",
            Self::HealClip => "heal_clip",
            Self::ChaosReshaped => "chaos_reshaped",
        };

        write!(f, "{name}")
    }
}
