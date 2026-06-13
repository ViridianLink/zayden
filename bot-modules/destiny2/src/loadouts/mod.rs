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
pub mod weapons;

use std::fmt::{Display, Formatter, Write};
use std::{fmt, iter};

use builds::{ARC_TITAN, SOLAR_TITAN, VOID_TITAN};
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

const BUILDS: [Loadout<'_>; 3] = [ARC_TITAN, VOID_TITAN, SOLAR_TITAN];
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
        gauntlets: titan::Gauntlets,
        plate: titan::Plate,
        greaves: titan::Greaves,
        mark: titan::Mark,
    },
}

impl Armour {
    #[must_use]
    pub fn items(self) -> [Box<dyn ArmourItem>; 5] {
        match self {
            Self::Titan {
                helmet,
                gauntlets,
                plate: chest,
                greaves: legs,
                mark: class,
            } => [
                Box::new(helmet),
                Box::new(gauntlets),
                Box::new(chest),
                Box::new(legs),
                Box::new(class),
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

// #[derive(Clone, Copy)]
// pub struct Armour {
//     name: ArmourName,
//     mods: [Mod; 3],
// }

// impl Armour {
//     #[must_use]
//     pub const fn new(name: ArmourName, mods: [Mod; 3]) -> Self {
//         Self { name, mods }
//     }

// #[derive(Clone, Copy)]
// pub enum ArmourName {
//     MelasPanoplia,
//     WormgodCaress,
//     BushidoHelm,
//     BushidoPlate,
//     BushidoGreaves,
//     BushidoMark,
//     BushidoCowl,
//     BushidoGrips,
//     LastDisciplineVest,
//     LastDisciplineStrides,
//     CollectivePsycheCover,
//     CollectivePsycheGloves,
//     StarfireProtocol,
//     CollectivePsycheBoots,
//     CollectivePsycheBond,
//     LustrousHelm,
//     LustrousPlate,
//     LustrousGreaves,
//     LustrousMark,
//     AnInsurmountableSkullfort,
//     CollectivePsycheGauntlets,
//     CollectivePsychePlate,
//     CollectivePsycheGreaves,
//     CollectivePsycheMark,
//     MaskOfBakris,
//     Relativism((&'static str, &'static str)),
//     BushidoVest,
//     LastDisciplineCloak,
//     CollectivePsycheCasque,
//     CollectivePsycheCuirass,
//     CollectivePsycheSleeves,
//     CollectivePsycheStrides,
//     CollectivePsycheHelm,
//     WishfulIgnorance,
//     GiftedConviction,
//     HunterHelmet,
//     HunterArms,
//     HunterLegs,
//     Cloak,
//     AionAdapterGloves,
//     AionAdapterRobes,
//     AionAdapterBoots,
//     AionAdapterBond,
//     VeritysBrow,
//     AionAdapterHood,
//     AIONRenewalRobes,
//     Swarmers,
//     AIONRenewalBond,
//     WarlockHood,
//     WarlockGloves,
//     WarlockRobes,
//     WarlockBoots,
//     Solipsism((&'static str, &'static str)),
//     TechsecGloves,
//     TechsecVestment,
//     TwofoldCrownBoots,
//     TwofoldCrownBond,
// }

// impl Display for ArmourName {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let url = match self {
//             Self::MelasPanoplia => {
//                 "https://www.bungie.net/common/destiny2_content/icons/8546b88189f69d88f8efa3d258f67026.jpg"
//             },
//             Self::WormgodCaress => {
//                 "https://www.bungie.net/common/destiny2_content/icons/f93fb202061de21b42138c9348359d27.jpg"
//             },
//             Self::BushidoHelm => {
//                 "https://www.bungie.net/common/destiny2_content/icons/9879c7eda4c3bcb56712a964f57717e9.jpg"
//             },
//             Self::BushidoPlate => {
//                 "https://www.bungie.net/common/destiny2_content/icons/35c2f575bf2584e4e9729bcbb5c62a85.jpg"
//             },
//             Self::BushidoGreaves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/aaab3065cf9f92898ef641da58b2585b.jpg"
//             },
//             Self::BushidoMark => {
//                 "https://www.bungie.net/common/destiny2_content/icons/9376932f07459b7a5858dfa73730c84c.jpg"
//             },
//             Self::BushidoCowl => {
//                 "https://www.bungie.net/common/destiny2_content/icons/9c38bcbbb84005d4c1bd6b9184a58571.jpg"
//             },
//             Self::BushidoGrips => {
//                 "https://www.bungie.net/common/destiny2_content/icons/8e948205999822eb4ba7933ef05ba56c.jpg"
//             },
//             Self::LastDisciplineVest => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1f3f5870b6e1163d589da044c48a20ca.jpg"
//             },
//             Self::LastDisciplineStrides => {
//                 "https://www.bungie.net/common/destiny2_content/icons/db74932fddacc7a8a98844f2480e4a7f.jpg"
//             },
//             Self::CollectivePsycheCover => {
//                 "https://www.bungie.net/common/destiny2_content/icons/41157409d6cfd4da8f44f36f1f7d7e40.jpg"
//             },
//             Self::CollectivePsycheGloves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/fec9d8ed57853226cc031d6ffed9a70c.jpg"
//             },
//             Self::StarfireProtocol => {
//                 "https://www.bungie.net/common/destiny2_content/icons/707703c3e72776cbf463a2d6427f5b43.jpg"
//             },
//             Self::CollectivePsycheBoots => {
//                 "https://www.bungie.net/common/destiny2_content/icons/8d9a8b0ba16b2d0bc9fa5ab1266ecb9b.jpg"
//             },
//             Self::CollectivePsycheBond => {
//                 "https://www.bungie.net/common/destiny2_content/icons/19e70cd67f1f361003bcdaa59952fbab.jpg"
//             },
//             Self::LustrousHelm => {
//                 "https://www.bungie.net/common/destiny2_content/icons/67d2e115db35baf3509a7a54d2d620be.jpg"
//             },
//             Self::LustrousPlate => {
//                 "https://www.bungie.net/common/destiny2_content/icons/b82af1a81e8fdf6f3101c3ec85116387.jpg"
//             },
//             Self::LustrousGreaves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/775e22c8c987b15e3834efcb35c84996.jpg"
//             },
//             Self::LustrousMark => {
//                 "https://www.bungie.net/common/destiny2_content/icons/7e2d5b6b4bfbc99b00f1447836ba6795.jpg"
//             },
//             Self::AnInsurmountableSkullfort => {
//                 "https://www.bungie.net/common/destiny2_content/icons/b734daf76fba2c835ba58ebca84c1d61.jpg"
//             },
//             Self::CollectivePsycheGauntlets => {
//                 "https://www.bungie.net/common/destiny2_content/icons/98aeaf66c0dd814cb1d72ef4b1c725bc.jpg"
//             },
//             Self::CollectivePsychePlate => {
//                 "https://www.bungie.net/common/destiny2_content/icons/edbc60a615bd223bfe4cd30c46a58d49.jpg"
//             },
//             Self::CollectivePsycheGreaves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/d8a5bd616380eff7886b55cf5a496111.jpg"
//             },
//             Self::CollectivePsycheMark => {
//                 "https://www.bungie.net/common/destiny2_content/icons/845d32ecf59ca0eea8c54cf9e108eb3d.jpg"
//             },
//             Self::MaskOfBakris => {
//                 "https://www.bungie.net/common/destiny2_content/icons/c753c91b8ff629cc60e835aebc8da958.jpg"
//             },
//             Self::Relativism(_) => {
//                 "https://www.bungie.net/common/destiny2_content/icons/e4acc5bd83081bcf82f8e7c8905b58c4.jpg"
//             },
//             Self::BushidoVest => {
//                 "https://www.bungie.net/common/destiny2_content/icons/982d331f44b50ab074c856effdf4ac23.jpg"
//             },
//             Self::LastDisciplineCloak => {
//                 "https://www.bungie.net/common/destiny2_content/icons/da32491871e833d20955b2f055d59ab6.jpg"
//             },
//             Self::CollectivePsycheCasque => {
//                 "https://www.bungie.net/common/destiny2_content/icons/2ad2c64c11a5b3f86382cfb94517a561.jpg"
//             },
//             Self::CollectivePsycheCuirass => {
//                 "https://www.bungie.net/common/destiny2_content/icons/0aa178e78bb12e1962e183b2696f9f92.jpg"
//             },
//             Self::CollectivePsycheSleeves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/f64ecc6277d8a4df49813adb071e4dbb.jpg"
//             },
//             Self::CollectivePsycheStrides => {
//                 "https://www.bungie.net/common/destiny2_content/icons/7b661a41864b375de2a3d4b299cd8a99.jpg"
//             },
//             Self::CollectivePsycheHelm => {
//                 "https://www.bungie.net/common/destiny2_content/icons/eded09222a4d5bab546ad3cf04d24bf3.jpg"
//             },
//             Self::WishfulIgnorance => {
//                 "https://www.bungie.net/common/destiny2_content/icons/4a0247f3edb22758ba945e6ba341721b.jpg"
//             },
//             Self::GiftedConviction => {
//                 "https://www.bungie.net/common/destiny2_content/icons/a8f8856e51daa04775b2d510b2ca12f1.jpg"
//             },
//             Self::HunterHelmet => {
//                 "https://www.bungie.net/common/destiny2_content/icons/d2abc2257f85934b8ff763e563f02cd9.jpg"
//             },
//             Self::HunterArms => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1cfe58452f5dae674b7f6d0f816e9592.jpg"
//             },
//             Self::HunterLegs => {
//                 "https://www.bungie.net/common/destiny2_content/icons/9cc3f7461305a1ece9f91f5a25d9e7a9.jpg"
//             },
//             Self::Cloak => {
//                 "https://www.bungie.net/common/destiny2_content/icons/363fd4e1311408d0f5400f6d9579cf2f.jpg"
//             },
//             Self::VeritysBrow => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1eaa3f087b696caa6e8308e65883fb22.jpg"
//             },
//             Self::AionAdapterGloves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/3300af1f577f999d59651d10ee16df52.jpg"
//             },
//             Self::AionAdapterRobes => {
//                 "https://www.bungie.net/common/destiny2_content/icons/6e431ef7eb277ca27ac4204b32cf03a1.jpg"
//             },
//             Self::AionAdapterBoots => {
//                 "https://www.bungie.net/common/destiny2_content/icons/09a5ab08b8f9f258fe5357a67188a3c9.jpg"
//             },
//             Self::AionAdapterBond => {
//                 "https://www.bungie.net/common/destiny2_content/icons/ed84553c654e3c5a74c83efa5354ffd8.jpg"
//             },
//             Self::AionAdapterHood => {
//                 "https://www.bungie.net/common/destiny2_content/icons/fc6f5043c2e35c80fa87cf557e105cb7.jpg"
//             },
//             Self::AIONRenewalRobes => {
//                 "https://www.bungie.net/common/destiny2_content/icons/90c55f512d646cf5100af428a194fdd0.jpg"
//             },
//             Self::Swarmers => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1267deeabc5cb6863332d4ec05b5afc8.jpg"
//             },
//             Self::AIONRenewalBond => {
//                 "https://www.bungie.net/common/destiny2_content/icons/2d4242012ce9246f3289dafddfa9dd60.jpg"
//             },
//             Self::WarlockHood => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1cb2285f74ece98b03e170a3f8d9abdc.jpg"
//             },
//             Self::WarlockGloves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/bfece8a540293e1ac584d894caaa7258.jpg"
//             },
//             Self::WarlockRobes => {
//                 "https://www.bungie.net/common/destiny2_content/icons/9fc0d6f0828aea5abe2f13354c6e63b5.jpg"
//             },
//             Self::WarlockBoots => {
//                 "https://www.bungie.net/common/destiny2_content/icons/1c3ae268b2f129c252f0609fe52b8028.jpg"
//             },
//             Self::Solipsism(_) => {
//                 "https://www.bungie.net/common/destiny2_content/icons/5d657945620203cc8a7b5ade47e6e12a.jpg"
//             },
//             Self::TechsecGloves => {
//                 "https://www.bungie.net/common/destiny2_content/icons/fe1fcf9002c3148bd933801a43613102.jpg"
//             },
//             Self::TechsecVestment => {
//                 "https://www.bungie.net/common/destiny2_content/icons/2e53661958423ed5bfd1fcdd3d2f0ec9.jpg"
//             },
//             Self::TwofoldCrownBoots => {
//                 "https://www.bungie.net/common/destiny2_content/icons/190b1833593db2263bf8318e59f1db31.jpg"
//             },
//             Self::TwofoldCrownBond => {
//                 "https://www.bungie.net/common/destiny2_content/icons/efcc8e332f9d5a3c8ef4b4d0511f7673.jpg"
//             },
//         };

//         write!(f, "{url}")
//     }
// }

// impl Debug for ArmourName {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let name = match self {
//             Self::MelasPanoplia => "Melas Panoplia",
//             Self::WormgodCaress => "Wormgod Caress",
//             Self::BushidoHelm => "Bushido Helm",
//             Self::BushidoPlate => "Bushido Plate",
//             Self::BushidoGreaves => "Bushido Greaves",
//             Self::BushidoMark => "Bushido Mark",
//             Self::BushidoCowl => "Bushido Cowl",
//             Self::BushidoGrips => "Bushido Grips",
//             Self::LastDisciplineVest => "Last Discipline Vest",
//             Self::LastDisciplineStrides => "Last Discipline Strides",
//             Self::CollectivePsycheCover => "Collective Psyche Cover",
//             Self::CollectivePsycheGloves => "Collective Psyche Gloves",
//             Self::StarfireProtocol => "Starfire Protocol",
//             Self::CollectivePsycheBoots => "Collective Psyche Boots",
//             Self::CollectivePsycheBond => "Collective PsycheBond",
//             Self::LustrousHelm => "Lustrous Helm",
//             Self::LustrousPlate => "Lustrous Plate",
//             Self::LustrousGreaves => "Lustrous Greaves",
//             Self::LustrousMark => "Lustrous Mark",
//             Self::AnInsurmountableSkullfort => "An Insurmountable Skullfort",
//             Self::CollectivePsycheGauntlets => "Collective Psyche Gauntlets",
//             Self::CollectivePsychePlate => "Collective Psyche Plate",
//             Self::CollectivePsycheGreaves => "Collective Psyche Greaves",
//             Self::CollectivePsycheMark => "Collective Psyche Mark",
//             Self::MaskOfBakris => "Mask of Bakris",
//             Self::Relativism(perks) => {
//                 &format!("Relativism ({} + {})", perks.0, perks.1)
//             },
//             Self::BushidoVest => "Bushido Vest",
//             Self::LastDisciplineCloak => "Last Discipline Cloak",
//             Self::CollectivePsycheCasque => "Collective Psyche Casque",
//             Self::CollectivePsycheCuirass => "Collective Psyche Cuirass",
//             Self::CollectivePsycheSleeves => "Collective Psyche Sleeves",
//             Self::CollectivePsycheStrides => "Collective Psyche Strides",
//             Self::CollectivePsycheHelm => "Collective Psyche Helm",
//             Self::WishfulIgnorance => "Wishful Ignorance",
//             Self::GiftedConviction => "Gifted Conviction",
//             Self::HunterHelmet => "Any Helmet",
//             Self::HunterArms => "Any Arms",
//             Self::HunterLegs => "Any Legs",
//             Self::Cloak => "Any Cloak",
//             Self::AionAdapterGloves => "Aion Adapter Gloves",
//             Self::AionAdapterRobes => "Aion Adapter Robes",
//             Self::AionAdapterBoots => "Aion Adapter Boots",
//             Self::AionAdapterBond => "Aion Adapter Bond",
//             Self::VeritysBrow => "Verity's Brow",
//             Self::AionAdapterHood => "AION Adapter Hood",
//             Self::AIONRenewalRobes => "AION Renewal Robes",
//             Self::Swarmers => "Swarmers",
//             Self::AIONRenewalBond => "AION Renewal Bond",
//             Self::WarlockHood => "Any Hood",
//             Self::WarlockGloves => "Any Gloves",
//             Self::WarlockRobes => "Any Robe",
//             Self::WarlockBoots => "Any Boots",
//             Self::Solipsism(perks) => {
//                 &format!("Solipsism ({} + {})", perks.0, perks.1)
//             },
//             Self::TechsecGloves => "Techsec Gloves",
//             Self::TechsecVestment => "Techsec Vestment",
//             Self::TwofoldCrownBoots => "Twofold Crown Boots",
//             Self::TwofoldCrownBond => "Twofold Crown Bond",
//         };

//         write!(f, "{name}")
//     }
// }

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
