#![expect(
    clippy::redundant_pub_crate,
    reason = "items here are pub(crate) so they're usable from sibling \
              submodules (e.g. builds), but the enclosing modules are \
              intentionally private to keep them out of this crate's public \
              API; clippy::unreachable_pub would fire if these were `pub` \
              instead, so the two lints are mutually exclusive here"
)]

mod builds;
mod class;
mod fragments;
mod grenades;
mod hunter;
mod mode;
mod mods;
mod tag;
mod titan;
mod warlock;
mod weapons;

use std::fmt::{Display, Formatter, Write};
use std::{fmt, iter};

use builds::{
    ARC_TITAN,
    PRISMATIC_HUNTER,
    SOLAR_TITAN,
    SOLAR_WARLOCK,
    STASIS_HUNTER,
    STRAND_WARLOCK,
    VOID_HUNTER,
    VOID_TITAN,
    VOID_WARLOCK,
};
use class::DestinyClass;
use fragments::{
    ArcFragment,
    PrismaticFragment,
    SolarFragment,
    StasisFragment,
    StrandFragment,
    VoidFragment,
};
use grenades::{
    ArcGrenade,
    SolarGrenade,
    StasisGrenade,
    StrandGrenade,
    VoidGrenade,
};
use mode::Mode;
use mods::{ArmsMod, ChestMod, ClassItemMod, HelmetMod, LegsMod};
use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateActionRow,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
    EmojiId,
    MessageFlags,
    ResolvedOption,
    SeparatorSpacingSize,
};
use tag::Tag;
use tokio::sync::RwLock;
pub use weapons::{Perk, Weapon};
use zayden_core::{
    CoreError,
    EmojiCache,
    EmojiCacheData,
    EmojiResult,
    SubCommandOptions,
    sole_option,
};

use crate::Result;

const BUILDS: [Loadout<'_>; 9] = [
    ARC_TITAN,
    PRISMATIC_HUNTER,
    SOLAR_TITAN,
    SOLAR_WARLOCK,
    STASIS_HUNTER,
    STRAND_WARLOCK,
    VOID_HUNTER,
    VOID_TITAN,
    VOID_WARLOCK,
];
const DUPLICATE: EmojiId = EmojiId::new(1_395_743_560_388_706_374);

#[derive(Clone, Copy)]
pub struct Loadout<'a> {
    name: &'a str,
    class: DestinyClass,
    mode: Mode,
    tags: [Option<Tag>; 3],
    gear: Gear,
    artifact: Artifact,
    details: Details<'a>,
}

impl Loadout<'_> {
    pub fn register<'a>() -> CreateCommand<'a> {
        let mut warlock_builds = CreateCommandOption::new(
            CommandOptionType::String,
            "build",
            "Select the build",
        )
        .required(true);

        let mut titan_builds = CreateCommandOption::new(
            CommandOptionType::String,
            "build",
            "Select the build",
        )
        .required(true);

        let mut hunter_builds = CreateCommandOption::new(
            CommandOptionType::String,
            "build",
            "Select the build",
        )
        .required(true);

        for build in BUILDS {
            let name = format!("{} | {}", build.class.subclass(), build.name);
            let value = name.to_lowercase().replace([' ', '|'], "_");

            match build.class {
                DestinyClass::Warlock(_) => {
                    warlock_builds = warlock_builds.add_string_choice(name, value);
                },
                DestinyClass::Titan(_) => {
                    titan_builds = titan_builds.add_string_choice(name, value);
                },
                DestinyClass::Hunter(_) => {
                    hunter_builds = hunter_builds.add_string_choice(name, value);
                },
            }
        }

        CreateCommand::new("builds")
            .description("Destiny 2 Builds")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "warlock",
                    "Warlock Builds",
                )
                .add_sub_option(warlock_builds),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "titan",
                    "Titan Builds",
                )
                .add_sub_option(titan_builds),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "hunter",
                    "Hunter Builds",
                )
                .add_sub_option(hunter_builds),
            )
    }

    pub async fn run<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        parent_token: &str,
    ) -> Result<()> {
        let value: &str = {
            let options: SubCommandOptions<'_> = sole_option(&mut options)?;
            sole_option(&mut options.into_vec())?
        };

        let Some(build) = BUILDS.iter().copied().find(|build| {
            let subclass = build.class.subclass().to_string().to_lowercase();
            let name = build.name.to_lowercase().replace([' ', '|'], "_");
            format!("{subclass}___{name}").as_str() == value
        }) else {
            return Err(CoreError::MissingData("matching build").into());
        };

        let component = build.into_component::<Data>(ctx, parent_token).await;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![component]),
                ),
            )
            .await?;

        Ok(())
    }
}

impl<'a> Loadout<'a> {
    #[must_use]
    pub const fn tags(mut self, tags: [Option<Tag>; 3]) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub const fn artifact(mut self, artifact: Artifact) -> Self {
        self.artifact = artifact;
        self
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "emoji_cache borrows from the write guard; dropping it early would dangle"
    )]
    pub async fn into_component<Data: EmojiCacheData>(
        self,
        ctx: &Context,
        parent_token: &str,
    ) -> CreateComponent<'a> {
        let data_lock = ctx.data::<RwLock<Data>>();
        let mut data = data_lock.write().await;
        let emoji_cache = data.emojis_mut();

        let mut components = Vec::with_capacity(21);

        let subclass_btn = loop {
            match self.class.subclass().as_button(emoji_cache) {
                Ok(btn) => break btn,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let tags = CreateContainerComponent::ActionRow(CreateActionRow::buttons(
            iter::once(subclass_btn)
                .chain([CreateButton::from(self.mode)])
                .chain(self.tags.into_iter().flatten().map(CreateButton::from))
                .collect::<Vec<_>>(),
        ));

        let heading1 =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
                "-# {} {} Build",
                self.class.subclass(),
                self.class
            )));

        let mut details = format!("By {}", self.details.author);
        if let Some(url) = self.details.video {
            let _ = write!(details, " • [Video Guide]({url})");
        }

        let heading2 =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
                "# {}  •  {}  •  {}\n{details}",
                self.class,
                self.class.subclass().abilities().super_(),
                self.name
            )));

        let line_sep = CreateContainerComponent::Separator(
            CreateSeparator::new().divider(true),
        );

        let dim_link =
            CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
                CreateButton::new_link(self.details.dim_link)
                    .label("COPY DIM LINK")
                    .emoji(DUPLICATE),
            ]));

        let subclass_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(
                "### SUBCLASS\nSuper       Abilities                                       Aspects",
            ),
        );

        let aspects = loop {
            match self.aspects_str(emoji_cache) {
                Ok(s) => break s,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let super_emoji = loop {
            match self.super_emoji(emoji_cache) {
                Ok(emoji) => break emoji,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let class_emoji = loop {
            match self.class_emoji(emoji_cache) {
                Ok(emoji) => break emoji,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let jump_emoji = loop {
            match self.jump_emoji(emoji_cache) {
                Ok(emoji) => break emoji,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let melee_emoji = loop {
            match self.melee_emoji(emoji_cache) {
                Ok(emoji) => break emoji,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let grenade_emoji = loop {
            match self.grenade_emoji(emoji_cache) {
                Ok(emoji) => break emoji,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let subclass = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(format!(
                "# {super_emoji}    {class_emoji} {jump_emoji} {melee_emoji} {grenade_emoji}    {aspects}\n\nFragments",
            )),
        );

        let fragments_str = loop {
            match self.fragments_str(emoji_cache) {
                Ok(s) => break s,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let fragments = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new(format!("#{fragments_str}")),
        );

        let gear_and_mods_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new("### GEAR AND MODS"),
        );

        let weapons = loop {
            match self.weapon_components(emoji_cache) {
                Ok(v) => break v,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let armour = loop {
            match self.armour_components(emoji_cache) {
                Ok(v) => break v,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let stat_prio = loop {
            match self.stat_prio_str(emoji_cache) {
                Ok(s) => break s,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let artifact = loop {
            match self.artifact.try_to_str(emoji_cache) {
                Ok(s) => break s,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let mut misc_content = format!(
            "### Stats Priority\n#{stat_prio}\n### ARTIFACT PERKS\n# {artifact}",
        );

        if let Some(how_it_works) = self.details.how_it_works {
            misc_content.push_str("\n### HOW IT WORKS\n# ");
            misc_content.push_str(how_it_works);
        }

        let misc = CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
            misc_content,
        ));

        components.extend([
            heading1,
            heading2,
            tags,
            line_sep.clone(),
            dim_link,
            line_sep.clone(),
            subclass_heading,
            subclass,
            fragments,
            line_sep,
            gear_and_mods_heading,
        ]);
        components.extend(weapons);
        components.push(CreateContainerComponent::Separator(
            CreateSeparator::new().spacing(SeparatorSpacingSize::Large),
        ));
        components.extend(armour);
        components.push(misc);

        CreateComponent::Container(CreateContainer::new(components))
    }

    fn weapon_components(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<Vec<CreateContainerComponent<'a>>> {
        self.gear
            .weapons
            .into_iter()
            .flatten()
            .map(|weapon| {
                Ok(CreateContainerComponent::Section(CreateSection::new(
                    vec![weapon.into_section(emoji_cache)?],
                    CreateSectionAccessory::Thumbnail(weapon.into()),
                )))
            })
            .collect()
    }

    fn armour_components(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<Vec<CreateContainerComponent<'a>>> {
        self.gear
            .armour
            .items()
            .into_iter()
            .map(|armour| {
                Ok(CreateContainerComponent::Section(CreateSection::new(
                    vec![armour.as_section(emoji_cache)?],
                    CreateSectionAccessory::Thumbnail(armour.to_thumbnail()),
                )))
            })
            .collect()
    }

    fn aspects_str(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        let s = self
            .class
            .subclass()
            .aspects()
            .into_iter()
            .map(|a| a.to_string())
            .map(|s| emoji_cache.emoji_str(&s))
            .collect::<EmojiResult<Vec<String>>>()?
            .join(" ");

        Ok(s)
    }

    fn super_emoji(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        emoji_cache
            .emoji_str(&self.class.subclass().abilities().super_().to_string())
    }

    fn class_emoji(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        emoji_cache.emoji_str(&self.class.subclass().abilities().class().to_string())
    }

    fn jump_emoji(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        emoji_cache.emoji_str(&self.class.subclass().abilities().jump().to_string())
    }

    fn melee_emoji(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        emoji_cache.emoji_str(&self.class.subclass().abilities().melee().to_string())
    }

    fn grenade_emoji(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        emoji_cache
            .emoji_str(&self.class.subclass().abilities().grenade().to_string())
    }

    fn fragments_str(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        let s = self
            .class
            .subclass()
            .aspects()
            .map(|a| a.fragments())
            .as_flattened()
            .iter()
            .flatten()
            .map(ToString::to_string)
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<EmojiResult<String>>()?;

        Ok(s)
    }

    fn stat_prio_str(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        let s = self
            .gear
            .stats_priority
            .into_iter()
            .enumerate()
            .map(|(i, stat)| {
                let emoji = emoji_cache.emoji_str(&stat.to_string())?;
                let value = stat.value();

                let s =
                    if value < 200 { format!("`{value}` {emoji}") } else { emoji };

                let s = if i == 0 { format!(" {s}") } else { format!(" → {s}") };

                Ok(s)
            })
            .collect::<EmojiResult<String>>()?;

        Ok(s)
    }
}

impl Display for Loadout<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} | {}", self.class.subclass(), self.name)
    }
}

pub trait Subclass: Display + Send {
    fn abilities(&self) -> Box<dyn Abilities>;

    fn aspects(&self) -> [Box<dyn Aspect>; 2];

    fn as_button<'a>(
        &self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateButton<'a>> {
        let name = self.to_string();
        let name_lower = name.to_lowercase();

        let emoji = emoji_cache.emoji(&name_lower)?;

        let button = CreateButton::new(name_lower)
            .label(name)
            .emoji(emoji)
            .style(ButtonStyle::Secondary);

        Ok(button)
    }
}

pub trait Abilities {
    fn super_(&self) -> Box<dyn Display>;
    fn class(&self) -> Box<dyn Display>;
    fn jump(&self) -> Box<dyn Display>;
    fn melee(&self) -> Box<dyn Display>;
    fn grenade(&self) -> Box<dyn Display>;
}

pub trait Aspect: Display {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3];
}

fn box_display<T: Display + 'static>(value: T) -> Box<dyn Display> {
    Box::new(value)
}

fn box_aspect<T: Aspect + 'static>(value: T) -> Box<dyn Aspect> {
    Box::new(value)
}

// #[derive(Clone, Copy)]
// pub enum Super {
//     BurningMaul,
//     GoldenGunMarksman,
//     SongOfFlame,
//     Thundercrash,
//     GatheringStorm,
//     Bladefury,
//     NovaBombCataclysm,
//     Needlestorm,
//     ChaosReach,
// }

// impl Display for Super {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let name = match self {
//             Self::BurningMaul => "Burning Maul",
//             Self::GoldenGunMarksman => "Golden Gun: Marksman",
//             Self::SongOfFlame => "Song of Flame",
//             Self::Thundercrash => "Thundercrash",
//             Self::GatheringStorm => "Gathering Storm",
//             Self::Bladefury => "Bladefury",
//             Self::NovaBombCataclysm => "Nova Bomb: Cataclysm",
//             Self::Needlestorm => "Needlestorm",
//             Self::ChaosReach => "Chaos Reach",
//         };

//         write!(f, "{name}")
//     }
// }

// impl Debug for Super {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let name = match self {
//             Self::BurningMaul => "burning_maul",
//             Self::GoldenGunMarksman => "golden_gun__marksman",
//             Self::SongOfFlame => "song_of_flame",
//             Self::Thundercrash => "thundercrash",
//             Self::GatheringStorm => "gathering_storm",
//             Self::Bladefury => "bladefury",
//             Self::NovaBombCataclysm => "nova_bomb_cataclysm",
//             Self::Needlestorm => "needlestorm",
//             Self::ChaosReach => "chaos_reach",
//         };

//         write!(f, "{name}")
//     }
// }

#[derive(Clone, Copy)]
pub enum ClassAbility {
    MarksmansDodge,
    PhoenixDive,
    Thruster,
    GamblersDodge,
    HealingRift,
    EmpoweringRift,
}

impl Display for ClassAbility {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::MarksmansDodge => "marksmans_dodge",
            Self::PhoenixDive => "phoenix_dive",
            Self::Thruster => "thruster",
            Self::GamblersDodge => "gamblers_dodge",
            Self::HealingRift => "healing_rift",
            Self::EmpoweringRift => "empowering_rift",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Jump {
    CatapultLift,
    Triple,
    BurstGlide,
}

impl Display for Jump {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::CatapultLift => "catapult_lift",
            Self::Triple => "triple_jump",
            Self::BurstGlide => "burst_glide",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Melee {
    ThrowingHammer,
    ThreadedSpike,
    IncineratorSnap,
    Thunderclap,
    CombinationBlow,
    FrenziedBlade,
    PocketSingularity,
    ArcaneNeedle,
    BallLightning,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ThrowingHammer => "throwing_hammer",
            Self::ThreadedSpike => "threaded_spike",
            Self::IncineratorSnap => "incinerator_snap",
            Self::Thunderclap => "thunderclap",
            Self::CombinationBlow => "combination_blow",
            Self::FrenziedBlade => "frenzied_blade",
            Self::PocketSingularity => "pocket_singularity",
            Self::ArcaneNeedle => "arcane_needle",
            Self::BallLightning => "ball_lightning",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Grenade {
    Healing,
    Grapple,
    Fusion,
    Shackle,
    Flux,
    Magnetic,
    Threadling,
    Vortex,
    Pulse,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Healing => "healing_grenade",
            Self::Grapple => "grapple_grenade",
            Self::Fusion => "fusion_grenade",
            Self::Shackle => "shackle_grenade",
            Self::Flux => "flux_grenade",
            Self::Magnetic => "magnetic_grenade",
            Self::Threadling => "threadling_grenade",
            Self::Vortex => "vortex_grenade",
            Self::Pulse => "pulse_grenade",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Fragment {
    EmberOfAshes,
    EmberOfEmpyrean,
    EmberOfSearing,
    EmberOfTorches,
    FacetOfHope,
    FacetOfProtection,
    FacetOfPurpose,
    FacetOfDawn,
    FacetOfBlessing,
    EmberOfMercy,
    FacetOfCourage,
    FacetOfAwakening,
    FacetOfSacrifice,
    SparkOfResistance,
    SparkOfAmplitude,
    SparkOfFrequency,
    SparkOfDischarge,
    ThreadOfFury,
    ThreadOfWarding,
    ThreadOfTransmutation,
    ThreadOfGeneration,
    SparkOfIons,
    SparkOfFeedback,
    EchoOfPersistence,
    EchoOfInstability,
    EchoOfExpulsion,
    EchoOfVigilance,
    ThreadOfMind,
    ThreadOfEvolution,
    FacetOfDominance,
    SparkOfShock,
    SparkOfBeacons,
}

impl Display for Fragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::EmberOfAshes => "ember_of_ashes",
            Self::EmberOfEmpyrean => "ember_of_empyrean",
            Self::EmberOfSearing => "ember_of_searing",
            Self::EmberOfTorches => "ember_of_torches",
            Self::EmberOfMercy => "ember_of_mercy",
            Self::FacetOfHope => "facet_of_hope",
            Self::FacetOfProtection => "facet_of_protection",
            Self::FacetOfPurpose => "facet_of_purpose",
            Self::FacetOfDawn => "facet_of_dawn",
            Self::FacetOfBlessing => "facet_of_blessing",
            Self::FacetOfCourage => "facet_of_courage",
            Self::FacetOfAwakening => "facet_of_awakening",
            Self::FacetOfSacrifice => "facet_of_sacrifice",
            Self::FacetOfDominance => "facet_of_dominance",
            Self::SparkOfResistance => "spark_of_resistance",
            Self::SparkOfAmplitude => "spark_of_amplitude",
            Self::SparkOfFrequency => "spark_of_frequency",
            Self::SparkOfDischarge => "spark_of_discharge",
            Self::ThreadOfFury => "thread_of_fury",
            Self::ThreadOfWarding => "thread_of_warding",
            Self::ThreadOfTransmutation => "thread_of_transmutation",
            Self::ThreadOfGeneration => "thread_of_generation",
            Self::ThreadOfMind => "thread_of_mind",
            Self::ThreadOfEvolution => "thread_of_evolution",
            Self::SparkOfIons => "spark_of_ions",
            Self::SparkOfFeedback => "spark_of_feedback",
            Self::EchoOfPersistence => "echo_of_persistence",
            Self::EchoOfInstability => "echo_of_instability",
            Self::EchoOfExpulsion => "echo_of_expulsion",
            Self::EchoOfVigilance => "echo_of_vigilance",
            Self::SparkOfShock => "spark_of_shock",
            Self::SparkOfBeacons => "spark_of_beacons",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub struct Gear {
    weapons: [Option<Weapon>; 3],
    armour: Armour,
    stats_priority: [Stat; 6],
}

#[derive(Clone, Copy)]
pub enum Armour {
    Titan {
        helmet: titan::Helmet,
        arms: titan::Arms,
        chest: titan::Chest,
        legs: titan::Legs,
        mark: titan::Mark,
    },
    Warlock {
        helmet: warlock::Hood,
        gloves: warlock::Gloves,
        robes: warlock::Robes,
        boots: warlock::Boots,
        bond: warlock::Bond,
    },
    Hunter {
        helmet: hunter::Helmet,
        gauntlets: hunter::Gauntlets,
        vest: hunter::Vest,
        legs: hunter::Legs,
        cloak: hunter::Cloak,
    },
}

impl Armour {
    #[must_use]
    pub fn items(self) -> [Box<dyn ArmourItem>; 5] {
        match self {
            Self::Titan { helmet, arms, chest, legs, mark } => [
                Box::new(helmet),
                Box::new(arms),
                Box::new(chest),
                Box::new(legs),
                Box::new(mark),
            ],
            Self::Warlock { helmet, gloves: gauntlets, robes, boots, bond } => [
                Box::new(helmet),
                Box::new(gauntlets),
                Box::new(robes),
                Box::new(boots),
                Box::new(bond),
            ],
            Self::Hunter { helmet, gauntlets, vest, legs, cloak } => [
                Box::new(helmet),
                Box::new(gauntlets),
                Box::new(vest),
                Box::new(legs),
                Box::new(cloak),
            ],
        }
    }
}

pub trait ArmourItem: Display {
    fn mods(&self) -> [Box<dyn Display>; 3];

    fn as_text_display<'a>(
        &self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateTextDisplay<'a>> {
        let mods = self
            .mods()
            .into_iter()
            .map(|m| m.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<EmojiResult<String>>()?;

        let content = if mods.is_empty() {
            format!("**{self}**")
        } else {
            format!("**{self}**\n#{mods}")
        };

        Ok(CreateTextDisplay::new(content))
    }

    fn as_section<'a>(
        &self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateSectionComponent<'a>> {
        Ok(CreateSectionComponent::TextDisplay(self.as_text_display(emoji_cache)?))
    }

    fn to_thumbnail<'a>(&self) -> CreateThumbnail<'a> {
        CreateThumbnail::new(self.as_unfurled_media_item())
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a>;
}

#[derive(Clone, Copy)]
pub enum Stat {
    Health(u8),
    Melee(u8),
    Grenade(u8),
    Super(u8),
    Class(u8),
    Weapons(u8),
}

impl Stat {
    #[must_use]
    pub const fn value(&self) -> u8 {
        match *self {
            Self::Health(v)
            | Self::Melee(v)
            | Self::Grenade(v)
            | Self::Super(v)
            | Self::Class(v)
            | Self::Weapons(v) => v,
        }
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Health(_) => "health",
            Self::Melee(_) => "melee",
            Self::Grenade(_) => "grenade",
            Self::Super(_) => "super",
            Self::Class(_) => "class",
            Self::Weapons(_) => "weapons",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Artifact {
    QueensfoilCenser([Option<QueensfoilCenser>; 7]),
    TabletOfRuin([Option<TabletOfRuin>; 7]),
    ImplementOfCuriosity([Option<ImplementOfCuriosity>; 7]),
    EncryptedDataDisk([Option<EncryptedDataDisk>; 7]),
    NpaRepulsionRegulator([Option<NpaRepulsionRegulator>; 7]),
    HuntersJournal([Option<HuntersJournal>; 7]),
    SlayerBaronApothecarySatchel([Option<SlayerBaronApothecarySatchel>; 7]),
}

impl Artifact {
    pub fn try_to_str(self, emoji_cache: &EmojiCache) -> EmojiResult<String> {
        fn process_inner<I, T>(
            inner: I,
            cache: &EmojiCache,
        ) -> EmojiResult<Vec<String>>
        where
            I: IntoIterator,
            I::Item: IntoIterator<Item = T>,
            T: Display,
        {
            inner
                .into_iter()
                .flatten()
                .map(|ap| cache.emoji_str(&ap.to_string()))
                .collect()
        }

        let emojis = match self {
            Self::QueensfoilCenser(inner) => process_inner(inner, emoji_cache)?,
            Self::TabletOfRuin(inner) => process_inner(inner, emoji_cache)?,
            Self::ImplementOfCuriosity(inner) => process_inner(inner, emoji_cache)?,
            Self::EncryptedDataDisk(inner) => process_inner(inner, emoji_cache)?,
            Self::NpaRepulsionRegulator(inner) => process_inner(inner, emoji_cache)?,
            Self::HuntersJournal(inner) => process_inner(inner, emoji_cache)?,
            Self::SlayerBaronApothecarySatchel(inner) => {
                process_inner(inner, emoji_cache)?
            },
        };

        Ok(emojis.join(" "))
    }
}

#[derive(Clone, Copy)]
pub enum QueensfoilCenser {
    HordeShuttle,
    HailTheStorm,
    RaysOfPrecision,
    SoloOperative,
    ArgentOrdanance,
    FrigidGlare,
    ToShreds,

    UnravelingOrbs,
    PillarOfIce,
    RevitalizingBlast,
    AntiChampionNosecone,
    DragonsBite,
    CreepingChill,
    PerpetualDestruction,

    FeverAndChill,
    Torch,
    HeartOfTheFlame,
    Armoursmith,
    WishedIntoBeing,
    KindlingTrigger,
    BlastRadius,
}

impl Display for QueensfoilCenser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::HordeShuttle => "horde_shuttle",
            Self::HailTheStorm => "hail_the_storm",
            Self::RaysOfPrecision => "rays_of_precision",
            Self::SoloOperative => "solo_operative",
            Self::ArgentOrdanance => "argent_ordanance",
            Self::FrigidGlare => "frigid_glare",
            Self::ToShreds => "to_shreds",
            Self::UnravelingOrbs => "unraveling_orbs",
            Self::PillarOfIce => "pillar_of_ice",
            Self::RevitalizingBlast => "revitalizing_blast",
            Self::AntiChampionNosecone => "anti_champion_nosecone",
            Self::DragonsBite => "dragons_bite",
            Self::CreepingChill => "creeping_chill",
            Self::PerpetualDestruction => "perpetual_destruction",
            Self::FeverAndChill => "fever_and_chill",
            Self::Torch => "torch",
            Self::HeartOfTheFlame => "heart_of_the_flame",
            Self::Armoursmith => "armoursmith",
            Self::WishedIntoBeing => "wished_into_being",
            Self::KindlingTrigger => "kindling_trigger",
            Self::BlastRadius => "blast_radius",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum TabletOfRuin {
    ParticleReconstruction,
    ElementalSupercharge,
    HeavyOrdnanceRegeneration,
    DefibrillatingBlast,
    VoidFlux,
    LimitBreak,
    ToShreds,

    VileWeave,
    MalignedHarvest,
    Flashover,
    HordeShuttle,
    NoBell,
    HarshRefraction,
    GoldFromLead,

    HoldTheLine,
    Dielectric,
    VolatileMarksman,
    UnravelingOrbs,
    PhotonicFlare,
    ElementalSiphon,
    PerpetualDestruction,
}

impl Display for TabletOfRuin {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ParticleReconstruction => "particle_reconstruction",
            Self::ElementalSupercharge => "elemental_supercharge",
            Self::HeavyOrdnanceRegeneration => "heavy_ordnance_regeneration",
            Self::DefibrillatingBlast => "defibrillating_blast",
            Self::VoidFlux => "void_flux",
            Self::LimitBreak => "limit_break",
            Self::ToShreds => "to_shreds",
            Self::VileWeave => "vile_weave",
            Self::MalignedHarvest => "maligned_harvest",
            Self::Flashover => "flashover",
            Self::HordeShuttle => "horde_shuttle",
            Self::NoBell => "no_bell",
            Self::HarshRefraction => "harsh_refraction",
            Self::GoldFromLead => "gold_from_lead",
            Self::HoldTheLine => "hold_the_line",
            Self::Dielectric => "dielectric",
            Self::VolatileMarksman => "volatile_marksman",
            Self::UnravelingOrbs => "unraveling_orbs",
            Self::PhotonicFlare => "photonic_flare",
            Self::ElementalSiphon => "elemental_siphon",
            Self::PerpetualDestruction => "perpetual_destruction",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum ImplementOfCuriosity {
    RadiantShrapnel,
    Shieldcrush,
    ElementalOverdrive,
    TangledWeb,
    FrigidGlare,
    IronLordsVigor,
    ArgentQuiver,

    ThreadedBlast,
    CauterizedDarkness,
    ElementalDaze,
    ShoulderToShoulder,
    ElementalCoalescence,
    ThatFreshBulletSmell,
    EnergyAcceleration,

    FeverAndChill,
    ElementalBenevolence,
    FrostRenewal,
    HordeShuttle,
    RefreshThreads,
    PackTactics,
    SemiAutoStriker,
}

impl Display for ImplementOfCuriosity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::RadiantShrapnel => "radiant_shrapnel",
            Self::Shieldcrush => "shieldcrush",
            Self::ElementalOverdrive => "elemental_overdrive",
            Self::TangledWeb => "tangled_web",
            Self::FrigidGlare => "frigid_glare",
            Self::IronLordsVigor => "iron_lords_vigor",
            Self::ArgentQuiver => "argent_quiver",

            Self::ThreadedBlast => "threaded_blast",
            Self::CauterizedDarkness => "cauterized_darkness",
            Self::ElementalDaze => "elemental_daze",
            Self::ShoulderToShoulder => "shoulder_to_shoulder",
            Self::ElementalCoalescence => "elemental_coalescence",
            Self::ThatFreshBulletSmell => "that_fresh_bullet_smell",
            Self::EnergyAcceleration => "energy_acceleration",

            Self::FeverAndChill => "fever_and_chill",
            Self::ElementalBenevolence => "elemental_benevolence",
            Self::FrostRenewal => "frost_renewal",
            Self::HordeShuttle => "horde_shuttle",
            Self::RefreshThreads => "refresh_threads",
            Self::PackTactics => "pack_tactics",
            Self::SemiAutoStriker => "semi_auto_striker",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum EncryptedDataDisk {
    ElementalOrbsArc,
    KineticRupture,
    SnipersMeditation,
    SwordStormCombo,
    VoidInfestation,
    TazerTag,
    StickerShock,

    Dielectric,
    Armoursmith,
    CounterEnergy,
    CombinationArgentBlade,
    SingularityBlade,
    BlindingJolt,
    PrecisionEquity,

    PressTheAdvantage,
    KineticSynthesis,
    ReloadAtRange,
    Riposte,
    PowerFromPain,
    FierceProxemics,
    RapidRemedy,
}

impl Display for EncryptedDataDisk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ElementalOrbsArc => "elemental_orbs_arc",
            Self::KineticRupture => "kinetic_rupture",
            Self::SnipersMeditation => "snipers_meditation",
            Self::SwordStormCombo => "sword_storm_combo",
            Self::VoidInfestation => "void_infestation",
            Self::TazerTag => "tazer_tag",
            Self::StickerShock => "sticker_shock",
            Self::Dielectric => "dielectric",
            Self::Armoursmith => "armoursmith",
            Self::CounterEnergy => "counter_energy",
            Self::CombinationArgentBlade => "combination_argent_blade",
            Self::SingularityBlade => "singularity_blade",
            Self::BlindingJolt => "blinding_jolt",
            Self::PrecisionEquity => "precision_equity",
            Self::PressTheAdvantage => "press_the_advantage",
            Self::KineticSynthesis => "kinetic_synthesis",
            Self::ReloadAtRange => "reload_at_range",
            Self::Riposte => "riposte",
            Self::PowerFromPain => "power_from_pain",
            Self::FierceProxemics => "fierce_proxemics",
            Self::RapidRemedy => "rapid_remedy",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum NpaRepulsionRegulator {
    ConductiveCosmicNeedle,
    ShockAndAwe,
    Supernova,
    SquadGoals,
    LightningStrikesTwice,
    PassiveAggressiveGuard,
    VoidWeaponChanneling,

    StrandSoldier,
    SuppressingGlaive,
    ProtectiveBreach,
    CounterCharge,
    BricksFromBeyond,
    OverloadGrenades,
    TargetingAutoloader,

    ImprovedUnraveling,
    VolatileFlow,
    UntoTheBreach,
    AmpedUp,
    ThunderousRetort,
    SustainedFire,
    ShatterOrbs,
}

impl Display for NpaRepulsionRegulator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ConductiveCosmicNeedle => "conductive_cosmic_needle",
            Self::ShockAndAwe => "shock_and_awe",
            Self::Supernova => "supernova",
            Self::SquadGoals => "squad_goals",
            Self::LightningStrikesTwice => "lightning_strikes_twice",
            Self::PassiveAggressiveGuard => "passive_aggressive_guard",
            Self::VoidWeaponChanneling => "void_weapon_channeling",
            Self::StrandSoldier => "strand_soldier",
            Self::SuppressingGlaive => "suppressing_glaive",
            Self::ProtectiveBreach => "protective_breach",
            Self::CounterCharge => "counter_charge",
            Self::BricksFromBeyond => "bricks_from_beyond",
            Self::OverloadGrenades => "overload_grenades",
            Self::TargetingAutoloader => "targeting_autoloader",
            Self::ImprovedUnraveling => "improved_unraveling",
            Self::VolatileFlow => "volatile_flow",
            Self::UntoTheBreach => "unto_the_breach",
            Self::AmpedUp => "amped_up",
            Self::ThunderousRetort => "thunderous_retort",
            Self::SustainedFire => "sustained_fire",
            Self::ShatterOrbs => "shatter_orbs",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum HuntersJournal {
    PrismaticTransfer,
    ArgentBlade,
    ExpandingAbyss,
    Shieldcrush,
    Transference,
    SnipersMeditation,
    ShockAndAwe,

    CounterEnergy,
    BladeStamina,
    VoidHegemony,
    RadiantOrbs,
    BadAmplitude,
    SolarFulmination,
    TargetingAutoloader,

    ElementalSiphon,
    EnergyDiffusionSubstrate,
    CreepingChill,
    PressTheAdvantage,
    ThreadedBlast,
    IncendiaryRifleRounds,
    SustainedFire,
}

impl Display for HuntersJournal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::PrismaticTransfer => "prismatic_transfer",
            Self::ArgentBlade => "argent_blade",
            Self::ExpandingAbyss => "expanding_abyss",
            Self::Shieldcrush => "shieldcrush",
            Self::Transference => "transference",
            Self::SnipersMeditation => "snipers_meditation",
            Self::ShockAndAwe => "shock_and_awe",
            Self::CounterEnergy => "counter_energy",
            Self::BladeStamina => "blade_stamina",
            Self::VoidHegemony => "void_hegemony",
            Self::RadiantOrbs => "radiant_orbs",
            Self::BadAmplitude => "bad_amplitude",
            Self::SolarFulmination => "solar_fulmination",
            Self::TargetingAutoloader => "targeting_autoloader",
            Self::ElementalSiphon => "elemental_siphon",
            Self::EnergyDiffusionSubstrate => "energy_diffusion_substrate",
            Self::CreepingChill => "creeping_chill",
            Self::PressTheAdvantage => "press_the_advantage",
            Self::ThreadedBlast => "threaded_blast",
            Self::IncendiaryRifleRounds => "incendiary_rifle_rounds",
            Self::SustainedFire => "sustained_fire",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum SlayerBaronApothecarySatchel {
    BrainFreeze,
    Supernova,
    ConductiveCosmicCrystal,
    ServedCold,
    KineticImpacts,
    ArcCompounding,
    OldGodsRite,

    FrostRenewal,
    HailTheStorm,
    DebilitatingWave,
    WeakenedClear,
    RetinalBurn,
    CurativeOrbs,
    VoidRenewal,

    WindChill,
    CrystallineConverter,
    TotalCarnage,
    PowerFromPain,
    TraceEvidence,
    TheThickOfIt,
    KillingBreeze,
}

impl Display for SlayerBaronApothecarySatchel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::BrainFreeze => "brain_freeze",
            Self::Supernova => "supernova",
            Self::ConductiveCosmicCrystal => "conductive_cosmic_crystal",
            Self::ServedCold => "served_cold",
            Self::KineticImpacts => "kinetic_impacts",
            Self::ArcCompounding => "arc_compounding",
            Self::OldGodsRite => "old_gods_rite",
            Self::FrostRenewal => "frost_renewal",
            Self::HailTheStorm => "hail_the_storm",
            Self::DebilitatingWave => "debilitating_wave",
            Self::WeakenedClear => "weakened_clear",
            Self::RetinalBurn => "retinal_burn",
            Self::CurativeOrbs => "curative_orbs",
            Self::VoidRenewal => "void_renewal",
            Self::WindChill => "wind_chill",
            Self::CrystallineConverter => "crystalline_converter",
            Self::TotalCarnage => "total_carnage",
            Self::PowerFromPain => "power_from_pain",
            Self::TraceEvidence => "trace_evidence",
            Self::TheThickOfIt => "the_thick_of_it",
            Self::KillingBreeze => "killing_breeze",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub struct Details<'a> {
    author: &'a str,
    dim_link: &'a str,
    how_it_works: Option<&'a str>,
    video: Option<&'a str>,
}

impl<'a> Details<'a> {
    #[must_use]
    pub const fn new(author: &'a str, dim: &'a str) -> Self {
        Self { author, dim_link: dim, how_it_works: None, video: None }
    }

    #[must_use]
    pub const fn video(mut self, url: &'a str) -> Self {
        self.video = Some(url);
        self
    }
}
