use std::fmt::{Debug, Display};

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateComponent, CreateContainer,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateSection,
    CreateSectionAccessory, CreateSectionComponent, CreateSeparator, CreateTextDisplay,
    CreateThumbnail, CreateUnfurledMediaItem, EmojiId, MessageFlags, ResolvedOption, ResolvedValue,
    Spacing,
};

mod arc_hunter;
mod boss_prismatic_hunter;
mod general_prismatic_hunter;
mod prismatic_titan;
mod prismatic_warlock;
mod solar_titan;
mod solar_warlock;
mod strand_titan;
mod strand_warlock;
mod void_warlock;
use arc_hunter::ARC_HUNTER;
use boss_prismatic_hunter::BOSS_PRISMATIC_HUNTER;
use general_prismatic_hunter::GENERAL_PRISMATIC_HUNTER;
use prismatic_titan::PRISMATIC_TITAN;
use prismatic_warlock::PRISMATIC_WARLOCK;
use solar_titan::SOLAR_TITAN;
use solar_warlock::SOLAR_WARLOCK;
use strand_titan::STRAND_TITAN;
use strand_warlock::STRAND_WARLOCK;
use void_warlock::VOID_WARLOCK;

pub mod weapons;
use tokio::sync::RwLock;
pub use weapons::{Perk, Weapon};
use zayden_core::{EmojiCache, EmojiCacheData, EmojiResult};

const BUILDS: [Loadout; 10] = [
    ARC_HUNTER,
    GENERAL_PRISMATIC_HUNTER,
    BOSS_PRISMATIC_HUNTER,
    SOLAR_TITAN,
    STRAND_TITAN,
    PRISMATIC_TITAN,
    PRISMATIC_WARLOCK,
    SOLAR_WARLOCK,
    STRAND_WARLOCK,
    VOID_WARLOCK,
];
const DUPLICATE: EmojiId = EmojiId::new(1395743560388706374);

#[derive(Clone, Copy)]
pub struct Loadout<'a> {
    name: &'a str,
    class: DestinyClass,
    mode: Mode,
    tags: [Option<Tag>; 3],
    subclass: Subclass,
    gear: Gear<'a>,
    artifact: [Option<ArtifactPerk>; 8],
    details: Details<'a>,
}

impl Loadout<'_> {
    pub fn register<'a>() -> CreateCommand<'a> {
        let mut warlock_builds =
            CreateCommandOption::new(CommandOptionType::String, "build", "Select the build")
                .required(true);

        let mut titan_builds =
            CreateCommandOption::new(CommandOptionType::String, "build", "Select the build")
                .required(true);

        let mut hunter_builds =
            CreateCommandOption::new(CommandOptionType::String, "build", "Select the build")
                .required(true);

        for build in BUILDS {
            let name = format!("{} | {}", build.subclass.subclass, build.name);
            let value = name.to_lowercase().replace([' ', '|'], "_");

            match build.class {
                DestinyClass::Warlock => {
                    warlock_builds = warlock_builds.add_string_choice(name, value);
                }
                DestinyClass::Titan => {
                    titan_builds = titan_builds.add_string_choice(name, value);
                }
                DestinyClass::Hunter => {
                    hunter_builds = hunter_builds.add_string_choice(name, value);
                }
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
                CreateCommandOption::new(CommandOptionType::SubCommand, "titan", "Titan Builds")
                    .add_sub_option(titan_builds),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::SubCommand, "hunter", "Hunter Builds")
                    .add_sub_option(hunter_builds),
            )
    }

    pub async fn run<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        parent_token: &str,
    ) -> serenity::Result<()> {
        let ResolvedValue::SubCommand(options) = options.pop().unwrap().value else {
            unreachable!("Option must be a subcommand")
        };

        let ResolvedValue::String(value) = options.first().unwrap().value else {
            unreachable!("Option must be a string")
        };

        let build = BUILDS
            .iter()
            .copied()
            .find(|build| {
                let subclass = build.subclass.subclass.to_string().to_lowercase();
                let name = build.name.to_lowercase().replace([' ', '|'], "_");

                format!("{subclass}___{name}").as_str() == value
            })
            .unwrap();

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![build.into_component::<Data>(ctx, parent_token).await]),
                ),
            )
            .await?;

        Ok(())
    }
}

impl<'a> Loadout<'a> {
    pub const fn new(
        name: &'a str,
        class: DestinyClass,
        mode: Mode,
        subclass: Subclass,
        gear: Gear<'a>,
        details: Details<'a>,
    ) -> Self {
        Self {
            name,
            class,
            mode,
            tags: [None; 3],
            subclass,
            gear,
            artifact: [None; 8],
            details,
        }
    }

    pub const fn tags(mut self, tags: [Option<Tag>; 3]) -> Self {
        self.tags = tags;
        self
    }

    pub const fn artifact(mut self, artifact: [Option<ArtifactPerk>; 8]) -> Self {
        self.artifact = artifact;
        self
    }

    pub async fn into_component<Data: EmojiCacheData>(
        self,
        ctx: &Context,
        parent_token: &str,
    ) -> CreateComponent<'a> {
        let data_lock = ctx.data::<RwLock<Data>>();
        let mut data = data_lock.write().await;
        let emoji_cache = data
            .emojis_mut()
            .expect("Only 1 references should exist here");

        let mut components = Vec::with_capacity(21);

        let mut subclass_btn = self.subclass.subclass.into_button(emoji_cache);
        while let Err(name) = subclass_btn {
            emoji_cache.upload(ctx, parent_token, &name).await;
            subclass_btn = self.subclass.subclass.into_button(emoji_cache);
        }

        let tags = CreateComponent::ActionRow(CreateActionRow::buttons(
            [subclass_btn.unwrap()]
                .into_iter()
                .chain([CreateButton::from(self.mode)])
                .chain(self.tags.into_iter().flatten().map(CreateButton::from))
                .collect::<Vec<_>>(),
        ));

        let heading1 = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "-# {} {} Build",
            self.subclass.subclass, self.class
        )));

        let mut details = format!("By {}", self.details.author);
        if let Some(url) = self.details.video {
            details.push_str(&format!(" • [Video Guide]({url})"));
        }

        let heading2 = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "# {}  •  {}  •  {}\n{details}",
            self.class, self.subclass.abilities.super_, self.name
        )));

        let line_sep = CreateComponent::Separator(CreateSeparator::new(true));

        let dim_link = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            CreateButton::new_link(self.details.dim_link)
                .label("COPY DIM LINK")
                .emoji(DUPLICATE),
        ]));

        let subclass_heading = CreateComponent::TextDisplay(CreateTextDisplay::new(
            "### SUBCLASS\nSuper       Abilities                                       Aspects",
        ));

        let mut aspects = self.aspects_str(emoji_cache);
        while let Err(name) = aspects {
            emoji_cache.upload(ctx, parent_token, &name).await;
            aspects = self.aspects_str(emoji_cache)
        }

        let super_emoji = match self.super_emoji(emoji_cache) {
            Ok(emoji) => emoji,
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                self.super_emoji(emoji_cache).unwrap()
            }
        };

        let class_emoji = match self.class_emoji(emoji_cache) {
            Ok(emoji) => emoji,
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                self.class_emoji(emoji_cache).unwrap()
            }
        };

        let jump_emoji = match self.jump_emoji(emoji_cache) {
            Ok(emoji) => emoji,
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                self.jump_emoji(emoji_cache).unwrap()
            }
        };

        let melee_emoji = match self.melee_emoji(emoji_cache) {
            Ok(emoji) => emoji,
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                self.melee_emoji(emoji_cache).unwrap()
            }
        };

        let grenade_emoji = match self.grenade_emoji(emoji_cache) {
            Ok(emoji) => emoji,
            Err(name) => {
                emoji_cache.upload(ctx, parent_token, &name).await;
                self.grenade_emoji(emoji_cache).unwrap()
            }
        };

        let subclass = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "# {super_emoji}    {class_emoji} {jump_emoji} {melee_emoji} {grenade_emoji}    {}\n\nFragments",
            aspects.unwrap()
        )));

        let mut fragments = self.fragments_str(emoji_cache);
        while let Err(name) = fragments {
            emoji_cache.upload(ctx, parent_token, &name).await;
            fragments = self.fragments_str(emoji_cache)
        }

        let fragments = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "#{}",
            fragments.unwrap()
        )));

        let gear_and_mods_heading =
            CreateComponent::TextDisplay(CreateTextDisplay::new("### GEAR AND MODS"));

        let mut weapons = self.weapon_components(emoji_cache);
        while let Err(name) = weapons {
            emoji_cache.upload(ctx, parent_token, &name).await;
            weapons = self.weapon_components(emoji_cache)
        }

        let mut weapons = self.weapon_components(emoji_cache);
        while let Err(name) = weapons {
            emoji_cache.upload(ctx, parent_token, &name).await;
            weapons = self.weapon_components(emoji_cache)
        }

        let mut armour = self.armour_components(emoji_cache);
        while let Err(name) = armour {
            emoji_cache.upload(ctx, parent_token, &name).await;
            armour = self.armour_components(emoji_cache)
        }

        let mut stat_prio = self.stat_prio_str(emoji_cache);
        while let Err(name) = stat_prio {
            emoji_cache.upload(ctx, parent_token, &name).await;
            stat_prio = self.stat_prio_str(emoji_cache)
        }

        let mut artifact = self.artifact_str(emoji_cache);
        while let Err(name) = artifact {
            emoji_cache.upload(ctx, parent_token, &name).await;
            artifact = self.artifact_str(emoji_cache)
        }

        let mut misc_content = format!(
            "### Stats Priority\n#{}\n### ARTIFACT PERKS\n#{}",
            stat_prio.unwrap(),
            artifact.unwrap()
        );

        if let Some(how_it_works) = self.details.how_it_works {
            misc_content.push_str("\n### HOW IT WORKS\n# ");
            misc_content.push_str(how_it_works);
        }

        let misc = CreateComponent::TextDisplay(CreateTextDisplay::new(misc_content));

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
        components.extend(weapons.unwrap());
        components.push(CreateComponent::Separator(
            CreateSeparator::new(false).spacing(Spacing::Large),
        ));
        components.extend(armour.unwrap());
        components.push(misc);

        CreateComponent::Container(CreateContainer::new(components))
    }

    fn weapon_components(self, emoji_cache: &EmojiCache) -> EmojiResult<Vec<CreateComponent<'a>>> {
        self.gear
            .weapons
            .into_iter()
            .flatten()
            .map(|weapon| {
                Ok(CreateComponent::Section(CreateSection::new(
                    vec![weapon.into_section(emoji_cache)?],
                    CreateSectionAccessory::Thumbnail(weapon.into()),
                )))
            })
            .collect()
    }

    fn armour_components(self, emoji_cache: &EmojiCache) -> EmojiResult<Vec<CreateComponent<'a>>> {
        self.gear
            .armour
            .into_iter()
            .map(|armour| {
                Ok(CreateComponent::Section(CreateSection::new(
                    vec![armour.into_section(emoji_cache)?],
                    CreateSectionAccessory::Thumbnail(armour.into()),
                )))
            })
            .collect()
    }

    fn aspects_str(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        let s = self
            .subclass
            .aspects
            .into_iter()
            .map(|a| a.to_string())
            .map(|s| emoji_cache.emoji_str(&s))
            .collect::<Result<Vec<String>, String>>()?
            .join(" ");

        Ok(s)
    }

    fn super_emoji(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        emoji_cache.emoji_str(&format!("{:?}", self.subclass.abilities.super_))
    }

    fn class_emoji(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        emoji_cache.emoji_str(&self.subclass.abilities.class.to_string())
    }

    fn jump_emoji(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        emoji_cache.emoji_str(&self.subclass.abilities.jump.to_string())
    }

    fn melee_emoji(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        emoji_cache.emoji_str(&self.subclass.abilities.melee.to_string())
    }

    fn grenade_emoji(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        emoji_cache.emoji_str(&self.subclass.abilities.grenade.to_string())
    }

    fn fragments_str(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        let s = self
            .subclass
            .fragments
            .into_iter()
            .flatten()
            .map(|f| f.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<Result<String, String>>()?;

        Ok(s)
    }

    fn stat_prio_str(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        let s = self
            .gear
            .stats_priority
            .into_iter()
            .enumerate()
            .map(|(i, stat)| {
                let emoji = emoji_cache.emoji_str(&stat.to_string())?;
                let value = stat.value();

                let s = if value < 200 {
                    format!("({value}) {emoji}")
                } else {
                    emoji.to_string()
                };

                let s = if i == 0 {
                    format!(" {s}")
                } else {
                    format!(" → {s}")
                };

                Ok(s)
            })
            .collect::<Result<String, String>>()?;

        Ok(s)
    }

    fn artifact_str(self, emoji_cache: &EmojiCache) -> Result<String, String> {
        let s = self
            .artifact
            .into_iter()
            .flatten()
            .map(|ap| ap.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<Result<String, String>>()?;

        Ok(s)
    }
}

impl<'a> Display for Loadout<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | {}", self.subclass.subclass, self.name)
    }
}

#[derive(Clone, Copy)]
pub enum DestinyClass {
    Warlock,
    Titan,
    Hunter,
}

impl Display for DestinyClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DestinyClass::Warlock => write!(f, "Warlock"),
            DestinyClass::Titan => write!(f, "Titan"),
            DestinyClass::Hunter => write!(f, "Hunter"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Mode {
    All,
    PvE,
    PvP,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::All => write!(f, "All"),
            Mode::PvE => write!(f, "PvE"),
            Mode::PvP => write!(f, "PvP"),
        }
    }
}

impl<'a> From<Mode> for CreateButton<'a> {
    fn from(value: Mode) -> Self {
        CreateButton::new(format!("{value}"))
            .label(format!("{value}"))
            .style(ButtonStyle::Secondary)
    }
}

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Tag::EasyToPlay => "Easy To Play",
            Tag::BossDamage => "Boss Damage",
            Tag::AdClear => "Ad Clear",
            Tag::HighSurvivability => "High Survivability",
            Tag::Support => "Support",
            Tag::AntiChampion => "Anti-Champion",
            Tag::CasualPvP => "Casual PvP",
            Tag::CompetitivePvp => "Competitive PvP",
            Tag::Raids => "Raids",
            Tag::Dungeons => "Dungeons",
            Tag::MasterContent => "Master Content",
            Tag::GrandmasterNightfall => "Grandmaster Nightfall",
            Tag::Solo => "Solo",
            Tag::SuperFocused => "Super Focused",
            Tag::AbilityFocused => "Ability Focused",
            Tag::WeaponFocused => "Weapon Focused",
            Tag::HighDamage => "High Damage",
            Tag::EndGame => "End Game",
            Tag::CrowdControl => "Crowd Control",
        };

        write!(f, "{name}")
    }
}

impl<'a> From<Tag> for CreateButton<'a> {
    fn from(value: Tag) -> Self {
        CreateButton::new(format!("{value}"))
            .label(format!("{value}"))
            .style(ButtonStyle::Secondary)
    }
}

#[derive(Clone, Copy)]
pub struct Subclass {
    subclass: SubclassType,
    abilities: Abilities,
    aspects: [Aspect; 2],
    fragments: [Option<Fragment>; 5],
}

#[derive(Clone, Copy)]
pub enum SubclassType {
    Arc,
    Void,
    Strand,
    Stasis,
    Solar,
    Prismatic,
}

impl SubclassType {
    pub fn into_button<'a>(self, emoji_cache: &EmojiCache) -> EmojiResult<CreateButton<'a>> {
        let name = self.to_string();

        let emoji = emoji_cache.emoji(&name.to_lowercase())?;

        let button = CreateButton::new(name.to_lowercase())
            .label(name)
            .emoji(emoji)
            .style(ButtonStyle::Secondary);

        Ok(button)
    }
}

impl Display for SubclassType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubclassType::Arc => write!(f, "Arc"),
            SubclassType::Void => write!(f, "Void"),
            SubclassType::Strand => write!(f, "Strand"),
            SubclassType::Stasis => write!(f, "Stasis"),
            SubclassType::Solar => write!(f, "Solar"),
            SubclassType::Prismatic => write!(f, "Prismatic"),
        }
    }
}

impl Debug for SubclassType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arc => write!(f, "arc"),
            Self::Void => write!(f, "void"),
            Self::Strand => write!(f, "strand"),
            Self::Stasis => write!(f, "stasis"),
            Self::Solar => write!(f, "solar"),
            Self::Prismatic => write!(f, "prismatic"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Abilities {
    super_: Super,
    class: ClassAbility,
    jump: Jump,
    melee: Melee,
    grenade: Grenade,
}

#[derive(Clone, Copy)]
pub enum Super {
    BurningMaul,
    GoldenGunMarksman,
    SongOfFlame,
    Thundercrash,
    GatheringStorm,
    Bladefury,
    NovaBombCataclysm,
    Needlestorm,
}

impl Display for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Super::BurningMaul => "Burning Maul",
            Super::GoldenGunMarksman => "Golden Gun: Marksman",
            Super::SongOfFlame => "Song of Flame",
            Super::Thundercrash => "Thundercrash",
            Super::GatheringStorm => "Gathering Storm",
            Super::Bladefury => "Bladefury",
            Super::NovaBombCataclysm => "Nova Bomb: Cataclysm",
            Super::Needlestorm => "Needlestorm",
        };

        write!(f, "{name}")
    }
}

impl Debug for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Super::BurningMaul => "burning_maul",
            Super::GoldenGunMarksman => "golden_gun__marksman",
            Super::SongOfFlame => "song_of_flame",
            Super::Thundercrash => "thundercrash",
            Super::GatheringStorm => "gathering_storm",
            Super::Bladefury => "bladefury",
            Super::NovaBombCataclysm => "nova_bomb_cataclysm",
            Super::Needlestorm => "needlestorm",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum ClassAbility {
    RallyBarricade,
    MarksmansDodge,
    PhoenixDive,
    Thruster,
    GamblersDodge,
    HealingRift,
    EmpoweringRift,
}

impl Display for ClassAbility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ClassAbility::RallyBarricade => "rally_barricade",
            ClassAbility::MarksmansDodge => "marksmans_dodge",
            ClassAbility::PhoenixDive => "phoenix_dive",
            ClassAbility::Thruster => "thruster",
            ClassAbility::GamblersDodge => "gamblers_dodge",
            ClassAbility::HealingRift => "healing_rift",
            ClassAbility::EmpoweringRift => "empowering_rift",
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
}

impl Display for Melee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Melee::ThrowingHammer => "throwing_hammer",
            Melee::ThreadedSpike => "threaded_spike",
            Melee::IncineratorSnap => "incinerator_snap",
            Melee::Thunderclap => "thunderclap",
            Melee::CombinationBlow => "combination_blow",
            Melee::FrenziedBlade => "frenzied_blade",
            Melee::PocketSingularity => "pocket_singularity",
            Melee::ArcaneNeedle => "arcane_needle",
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
}

impl Display for Grenade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Grenade::Healing => "healing_grenade",
            Grenade::Grapple => "grapple_grenade",
            Grenade::Fusion => "fusion_grenade",
            Grenade::Shackle => "shackle_grenade",
            Grenade::Flux => "flux_grenade",
            Grenade::Magnetic => "magnetic_grenade",
            Grenade::Threadling => "threadling_grenade",
            Grenade::Vortex => "vortex_grenade",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Aspect {
    RoaringFlames,
    SolInvictus,
    Ascension,
    GunpowderGamble,
    TouchOfFlame,
    Hellion,
    Knockout,
    DiamondLance,
    TempestStrike,
    FlowState,
    StylishExecutioner,
    WintersShroud,
    BannerOfWar,
    FlechetteStorm,
    ChaosAccelerant,
    FeedTheVoid,
    Weavewalk,
    WeaversCall,
    LightningSurge,
}

impl Display for Aspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Aspect::RoaringFlames => "roaring_flames",
            Aspect::SolInvictus => "sol_invictus",
            Aspect::Ascension => "ascension",
            Aspect::GunpowderGamble => "gunpowder_gamble",
            Aspect::TouchOfFlame => "touch_of_flame",
            Aspect::Hellion => "hellion",
            Aspect::Knockout => "knockout",
            Aspect::DiamondLance => "diamond_lance",
            Aspect::TempestStrike => "tempest_strike",
            Aspect::FlowState => "flow_state",
            Aspect::StylishExecutioner => "stylish_executioner",
            Aspect::WintersShroud => "winters_shroud",
            Aspect::BannerOfWar => "banner_of_war",
            Aspect::FlechetteStorm => "flechette_storm",
            Aspect::ChaosAccelerant => "chaos_accelerant",
            Aspect::FeedTheVoid => "feed_the_void",
            Aspect::Weavewalk => "weavewalk",
            Aspect::WeaversCall => "weavers_call",
            Aspect::LightningSurge => "lightning_surge",
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
}

impl Display for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Fragment::EmberOfAshes => "ember_of_ashes",
            Fragment::EmberOfEmpyrean => "ember_of_empyrean",
            Fragment::EmberOfSearing => "ember_of_searing",
            Fragment::EmberOfTorches => "ember_of_torches",
            Fragment::EmberOfMercy => "ember_of_mercy",
            Fragment::FacetOfHope => "facet_of_hope",
            Fragment::FacetOfProtection => "facet_of_protection",
            Fragment::FacetOfPurpose => "facet_of_purpose",
            Fragment::FacetOfDawn => "facet_of_dawn",
            Fragment::FacetOfBlessing => "facet_of_blessing",
            Fragment::FacetOfCourage => "facet_of_courage",
            Fragment::FacetOfAwakening => "facet_of_awakening",
            Fragment::FacetOfSacrifice => "facet_of_sacrifice",
            Fragment::FacetOfDominance => "facet_of_dominance",
            Fragment::SparkOfResistance => "spark_of_resistance",
            Fragment::SparkOfAmplitude => "spark_of_amplitude",
            Fragment::SparkOfFrequency => "spark_of_frequency",
            Fragment::SparkOfDischarge => "spark_of_discharge",
            Fragment::ThreadOfFury => "thread_of_fury",
            Fragment::ThreadOfWarding => "thread_of_warding",
            Fragment::ThreadOfTransmutation => "thread_of_transmutation",
            Fragment::ThreadOfGeneration => "thread_of_generation",
            Fragment::ThreadOfMind => "thread_of_mind",
            Fragment::ThreadOfEvolution => "thread_of_evolution",
            Fragment::SparkOfIons => "spark_of_ions",
            Fragment::SparkOfFeedback => "spark_of_feedback",
            Fragment::EchoOfPersistence => "echo_of_persistence",
            Fragment::EchoOfInstability => "echo_of_instability",
            Fragment::EchoOfExpulsion => "echo_of_expulsion",
            Fragment::EchoOfVigilance => "echo_of_vigilance",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub struct Gear<'a> {
    weapons: [Option<Weapon<'a>>; 3],
    armour: [Armour; 5],
    stats_priority: [Stat; 6],
}

#[derive(Clone, Copy)]
pub struct Armour {
    name: ArmourName,
    mods: [Mod; 3],
}

impl Armour {
    pub const fn new(name: ArmourName, mods: [Mod; 3]) -> Self {
        Self { name, mods }
    }

    pub fn into_text_display<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateTextDisplay<'a>> {
        let mods = self
            .mods
            .into_iter()
            .map(|m| m.to_string())
            .map(|s| {
                let emoji = emoji_cache.emoji_str(&s)?;
                Ok(format!(" {emoji}"))
            })
            .collect::<EmojiResult<String>>()?;

        let content = if !mods.is_empty() {
            format!("**{:?}**\n#{mods}", self.name)
        } else {
            format!("**{:?}**", self.name)
        };

        Ok(CreateTextDisplay::new(content))
    }

    pub fn into_section<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateSectionComponent<'a>> {
        Ok(CreateSectionComponent::TextDisplay(
            self.into_text_display(emoji_cache)?,
        ))
    }
}

impl From<Armour> for CreateThumbnail<'_> {
    fn from(value: Armour) -> Self {
        CreateThumbnail::new(value.into())
    }
}

impl From<Armour> for CreateUnfurledMediaItem<'_> {
    fn from(value: Armour) -> Self {
        CreateUnfurledMediaItem::new(value.name.to_string())
    }
}

#[derive(Clone, Copy)]
pub enum ArmourName {
    MelasPanoplia,
    WormgodCaress,
    BushidoHelm,
    BushidoPlate,
    BushidoGreaves,
    BushidoMark,
    BushidoCowl,
    BushidoGrips,
    LastDisciplineVest,
    LastDisciplineStrides,
    CollectivePsycheCover,
    CollectivePsycheGloves,
    StarfireProtocol,
    CollectivePsycheBoots,
    CollectivePsycheBond,
    LustrousHelm,
    LustrousPlate,
    LustrousGreaves,
    LustrousMark,
    AnInsurmountableSkullfort,
    CollectivePsycheGauntlets,
    CollectivePsychePlate,
    CollectivePsycheGreaves,
    CollectivePsycheMark,
    MaskOfBakris,
    Relativism((&'static str, &'static str)),
    BushidoVest,
    LastDisciplineCloak,
    CollectivePsycheCasque,
    CollectivePsycheCuirass,
    CollectivePsycheSleeves,
    CollectivePsycheStrides,
    CollectivePsycheHelm,
    WishfulIgnorance,
    GiftedConviction,
    HunterHelmet,
    HunterArms,
    HunterLegs,
    Cloak,
    AionAdapterGloves,
    AionAdapterRobes,
    AionAdapterBoots,
    AionAdapterBond,
    VeritysBrow,
    AionAdapterHood,
    AIONRenewalRobes,
    Swarmers,
    AIONRenewalBond,
    WarlockHood,
    WarlockGloves,
    WarlockRobes,
    WarlockBoots,
    Solipsism((&'static str, &'static str)),
}

impl Display for ArmourName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = match self {
            ArmourName::MelasPanoplia => {
                "https://www.bungie.net/common/destiny2_content/icons/8546b88189f69d88f8efa3d258f67026.jpg"
            }
            ArmourName::WormgodCaress => {
                "https://www.bungie.net/common/destiny2_content/icons/f93fb202061de21b42138c9348359d27.jpg"
            }
            ArmourName::BushidoHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/9879c7eda4c3bcb56712a964f57717e9.jpg"
            }
            ArmourName::BushidoPlate => {
                "https://www.bungie.net/common/destiny2_content/icons/35c2f575bf2584e4e9729bcbb5c62a85.jpg"
            }
            ArmourName::BushidoGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/aaab3065cf9f92898ef641da58b2585b.jpg"
            }
            ArmourName::BushidoMark => {
                "https://www.bungie.net/common/destiny2_content/icons/9376932f07459b7a5858dfa73730c84c.jpg"
            }
            ArmourName::BushidoCowl => {
                "https://www.bungie.net/common/destiny2_content/icons/9c38bcbbb84005d4c1bd6b9184a58571.jpg"
            }
            ArmourName::BushidoGrips => {
                "https://www.bungie.net/common/destiny2_content/icons/8e948205999822eb4ba7933ef05ba56c.jpg"
            }
            ArmourName::LastDisciplineVest => {
                "https://www.bungie.net/common/destiny2_content/icons/1f3f5870b6e1163d589da044c48a20ca.jpg"
            }
            ArmourName::LastDisciplineStrides => {
                "https://www.bungie.net/common/destiny2_content/icons/db74932fddacc7a8a98844f2480e4a7f.jpg"
            }
            ArmourName::CollectivePsycheCover => {
                "https://www.bungie.net/common/destiny2_content/icons/41157409d6cfd4da8f44f36f1f7d7e40.jpg"
            }
            ArmourName::CollectivePsycheGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/fec9d8ed57853226cc031d6ffed9a70c.jpg"
            }
            ArmourName::StarfireProtocol => {
                "https://www.bungie.net/common/destiny2_content/icons/707703c3e72776cbf463a2d6427f5b43.jpg"
            }
            ArmourName::CollectivePsycheBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/8d9a8b0ba16b2d0bc9fa5ab1266ecb9b.jpg"
            }
            ArmourName::CollectivePsycheBond => {
                "https://www.bungie.net/common/destiny2_content/icons/19e70cd67f1f361003bcdaa59952fbab.jpg"
            }
            ArmourName::LustrousHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/67d2e115db35baf3509a7a54d2d620be.jpg"
            }
            ArmourName::LustrousPlate => {
                "https://www.bungie.net/common/destiny2_content/icons/b82af1a81e8fdf6f3101c3ec85116387.jpg"
            }
            ArmourName::LustrousGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/775e22c8c987b15e3834efcb35c84996.jpg"
            }
            ArmourName::LustrousMark => {
                "https://www.bungie.net/common/destiny2_content/icons/7e2d5b6b4bfbc99b00f1447836ba6795.jpg"
            }
            ArmourName::AnInsurmountableSkullfort => {
                "https://www.bungie.net/common/destiny2_content/icons/b734daf76fba2c835ba58ebca84c1d61.jpg"
            }
            ArmourName::CollectivePsycheGauntlets => {
                "https://www.bungie.net/common/destiny2_content/icons/98aeaf66c0dd814cb1d72ef4b1c725bc.jpg"
            }
            ArmourName::CollectivePsychePlate => {
                "https://www.bungie.net/common/destiny2_content/icons/edbc60a615bd223bfe4cd30c46a58d49.jpg"
            }
            ArmourName::CollectivePsycheGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/d8a5bd616380eff7886b55cf5a496111.jpg"
            }
            ArmourName::CollectivePsycheMark => {
                "https://www.bungie.net/common/destiny2_content/icons/845d32ecf59ca0eea8c54cf9e108eb3d.jpg"
            }
            ArmourName::MaskOfBakris => {
                "https://www.bungie.net/common/destiny2_content/icons/c753c91b8ff629cc60e835aebc8da958.jpg"
            }
            ArmourName::Relativism(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/e4acc5bd83081bcf82f8e7c8905b58c4.jpg"
            }
            ArmourName::BushidoVest => {
                "https://www.bungie.net/common/destiny2_content/icons/982d331f44b50ab074c856effdf4ac23.jpg"
            }
            ArmourName::LastDisciplineCloak => {
                "https://www.bungie.net/common/destiny2_content/icons/da32491871e833d20955b2f055d59ab6.jpg"
            }
            ArmourName::CollectivePsycheCasque => {
                "https://www.bungie.net/common/destiny2_content/icons/2ad2c64c11a5b3f86382cfb94517a561.jpg"
            }
            ArmourName::CollectivePsycheCuirass => {
                "https://www.bungie.net/common/destiny2_content/icons/0aa178e78bb12e1962e183b2696f9f92.jpg"
            }
            ArmourName::CollectivePsycheSleeves => {
                "https://www.bungie.net/common/destiny2_content/icons/f64ecc6277d8a4df49813adb071e4dbb.jpg"
            }
            ArmourName::CollectivePsycheStrides => {
                "https://www.bungie.net/common/destiny2_content/icons/7b661a41864b375de2a3d4b299cd8a99.jpg"
            }
            ArmourName::CollectivePsycheHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/eded09222a4d5bab546ad3cf04d24bf3.jpg"
            }
            ArmourName::WishfulIgnorance => {
                "https://www.bungie.net/common/destiny2_content/icons/4a0247f3edb22758ba945e6ba341721b.jpg"
            }
            ArmourName::GiftedConviction => {
                "https://www.bungie.net/common/destiny2_content/icons/a8f8856e51daa04775b2d510b2ca12f1.jpg"
            }
            ArmourName::HunterHelmet => {
                "https://www.bungie.net/common/destiny2_content/icons/d2abc2257f85934b8ff763e563f02cd9.jpg"
            }
            ArmourName::HunterArms => {
                "https://www.bungie.net/common/destiny2_content/icons/1cfe58452f5dae674b7f6d0f816e9592.jpg"
            }
            ArmourName::HunterLegs => {
                "https://www.bungie.net/common/destiny2_content/icons/9cc3f7461305a1ece9f91f5a25d9e7a9.jpg"
            }
            ArmourName::Cloak => {
                "https://www.bungie.net/common/destiny2_content/icons/363fd4e1311408d0f5400f6d9579cf2f.jpg"
            }
            ArmourName::VeritysBrow => {
                "https://www.bungie.net/common/destiny2_content/icons/1eaa3f087b696caa6e8308e65883fb22.jpg"
            }
            ArmourName::AionAdapterGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/3300af1f577f999d59651d10ee16df52.jpg"
            }
            ArmourName::AionAdapterRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/6e431ef7eb277ca27ac4204b32cf03a1.jpg"
            }
            ArmourName::AionAdapterBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/09a5ab08b8f9f258fe5357a67188a3c9.jpg"
            }
            ArmourName::AionAdapterBond => {
                "https://www.bungie.net/common/destiny2_content/icons/ed84553c654e3c5a74c83efa5354ffd8.jpg"
            }
            ArmourName::AionAdapterHood => {
                "https://www.bungie.net/common/destiny2_content/icons/fc6f5043c2e35c80fa87cf557e105cb7.jpg"
            }
            ArmourName::AIONRenewalRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/90c55f512d646cf5100af428a194fdd0.jpg"
            }
            ArmourName::Swarmers => {
                "https://www.bungie.net/common/destiny2_content/icons/1267deeabc5cb6863332d4ec05b5afc8.jpg"
            }
            ArmourName::AIONRenewalBond => {
                "https://www.bungie.net/common/destiny2_content/icons/2d4242012ce9246f3289dafddfa9dd60.jpg"
            }
            ArmourName::WarlockHood => {
                "https://www.bungie.net/common/destiny2_content/icons/1cb2285f74ece98b03e170a3f8d9abdc.jpg"
            }
            ArmourName::WarlockGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/bfece8a540293e1ac584d894caaa7258.jpg"
            }
            ArmourName::WarlockRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/9fc0d6f0828aea5abe2f13354c6e63b5.jpg"
            }
            ArmourName::WarlockBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/1c3ae268b2f129c252f0609fe52b8028.jpg"
            }
            ArmourName::Solipsism(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/5d657945620203cc8a7b5ade47e6e12a.jpg"
            }
        };

        write!(f, "{url}")
    }
}

impl Debug for ArmourName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ArmourName::MelasPanoplia => "Melas Panoplia",
            ArmourName::WormgodCaress => "Wormgod Caress",
            ArmourName::BushidoHelm => "Bushido Helm",
            ArmourName::BushidoPlate => "Bushido Plate",
            ArmourName::BushidoGreaves => "Bushido Greaves",
            ArmourName::BushidoMark => "Bushido Mark",
            ArmourName::BushidoCowl => "Bushido Cowl",
            ArmourName::BushidoGrips => "Bushido Grips",
            ArmourName::LastDisciplineVest => "Last Discipline Vest",
            ArmourName::LastDisciplineStrides => "Last Discipline Strides",
            ArmourName::CollectivePsycheCover => "Collective Psyche Cover",
            ArmourName::CollectivePsycheGloves => "Collective Psyche Gloves",
            ArmourName::StarfireProtocol => "Starfire Protocol",
            ArmourName::CollectivePsycheBoots => "Collective Psyche Boots",
            ArmourName::CollectivePsycheBond => "Collective PsycheBond",
            ArmourName::LustrousHelm => "Lustrous Helm",
            ArmourName::LustrousPlate => "Lustrous Plate",
            ArmourName::LustrousGreaves => "Lustrous Greaves",
            ArmourName::LustrousMark => "Lustrous Mark",
            ArmourName::AnInsurmountableSkullfort => "An Insurmountable Skullfort",
            ArmourName::CollectivePsycheGauntlets => "Collective Psyche Gauntlets",
            ArmourName::CollectivePsychePlate => "Collective Psyche Plate",
            ArmourName::CollectivePsycheGreaves => "Collective Psyche Greaves",
            ArmourName::CollectivePsycheMark => "Collective Psyche Mark",
            ArmourName::MaskOfBakris => "Mask of Bakris",
            ArmourName::Relativism(perks) => &format!("Relativism ({} + {})", perks.0, perks.1),
            ArmourName::BushidoVest => "Bushido Vest",
            ArmourName::LastDisciplineCloak => "Last Discipline Cloak",
            ArmourName::CollectivePsycheCasque => "Collective Psyche Casque",
            ArmourName::CollectivePsycheCuirass => "Collective Psyche Cuirass",
            ArmourName::CollectivePsycheSleeves => "Collective Psyche Sleeves",
            ArmourName::CollectivePsycheStrides => "Collective Psyche Strides",
            ArmourName::CollectivePsycheHelm => "Collective Psyche Helm",
            ArmourName::WishfulIgnorance => "Wishful Ignorance",
            ArmourName::GiftedConviction => "Gifted Conviction",
            ArmourName::HunterHelmet => "Any Helment",
            ArmourName::HunterArms => "Any Arms",
            ArmourName::HunterLegs => "Any Legs",
            ArmourName::Cloak => "Any Cloak",
            ArmourName::AionAdapterGloves => "Aion Adapter Gloves",
            ArmourName::AionAdapterRobes => "Aion Adapter Robes",
            ArmourName::AionAdapterBoots => "Aion Adapter Boots",
            ArmourName::AionAdapterBond => "Aion Adapter Bond",
            ArmourName::VeritysBrow => "Verity's Brow",
            ArmourName::AionAdapterHood => "AION Adapter Hood",
            ArmourName::AIONRenewalRobes => "AION Renewal Robes",
            ArmourName::Swarmers => "Swarmers",
            ArmourName::AIONRenewalBond => "AION Renewal Bond",
            ArmourName::WarlockHood => "Any Hood",
            ArmourName::WarlockGloves => "Any Gloves",
            ArmourName::WarlockRobes => "Any Robe",
            ArmourName::WarlockBoots => "Any Boots",
            ArmourName::Solipsism(perks) => &format!("Relativism ({} + {})", perks.0, perks.1),
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Mod {
    Empty,
    HandsOn,
    SpecialAmmoFinder,
    HarmonicSiphon,
    MeleeFont,
    HeavyHanded,
    StacksOnStacks,
    KineticScavenger,
    TimeDilation,
    Reaper,
    SpecialFinisher,
    AshesToAssets,
    SuperFont,
    VoidSiphon,
    Firepower,
    GrenadeFont,
    FocusingStrike,
    Recuperation,
    Invigoration,
    ClassFont,
    PowerfulAttraction,
    Innervation,
    StrandScavenger,
    Distribution,
    StasisSiphon,
    ImpactInduction,
    HarmonicLoader,
    ArcWeaponSurge,
    StasisWeaponSurge,
    StrandSiphon,
    Outreach,
    KineticSiphon,
    VoidAmmoGeneration,
    WeaponsFont,
    VoidScavenger,
    HarmonicScavenger,
    MomentumTransfer,
    StrandAmmoGeneration,
    Absolution,
}

impl Display for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Mod::Empty => "empty_mod",
            Mod::HandsOn => "hands_on",
            Mod::SpecialAmmoFinder => "special_ammo_finder",
            Mod::HarmonicSiphon => "harmonic_siphon",
            Mod::MeleeFont => "melee_font",
            Mod::HeavyHanded => "heavy_handed",
            Mod::StacksOnStacks => "stacks_on_stacks",
            Mod::KineticScavenger => "kinetic_scavenger",
            Mod::TimeDilation => "time_dilation",
            Mod::Reaper => "reaper",
            Mod::SpecialFinisher => "special_finisher",
            Mod::AshesToAssets => "ashes_to_assets",
            Mod::SuperFont => "super_font",
            Mod::VoidSiphon => "void_siphon",
            Mod::Firepower => "firepower",
            Mod::GrenadeFont => "grenade_font",
            Mod::FocusingStrike => "focusing_strike",
            Mod::Recuperation => "recuperation",
            Mod::Invigoration => "invigoration",
            Mod::ClassFont => "class_font",
            Mod::PowerfulAttraction => "powerful_attraction",
            Mod::Innervation => "innervation",
            Mod::StrandScavenger => "strand_scavenger",
            Mod::Distribution => "distribution",
            Mod::StasisSiphon => "stasis_siphon",
            Mod::ImpactInduction => "impact_induction",
            Mod::HarmonicLoader => "harmonic_loader",
            Mod::ArcWeaponSurge => "arc_weapon_surge",
            Mod::StasisWeaponSurge => "stasis_weapon_surge",
            Mod::StrandSiphon => "strand_siphon",
            Mod::Outreach => "outreach",
            Mod::KineticSiphon => "kinetic_siphon",
            Mod::VoidAmmoGeneration => "void_ammo_generation",
            Mod::WeaponsFont => "weapons_font",
            Mod::VoidScavenger => "void_scavenger",
            Mod::HarmonicScavenger => "harmonic_scavenger",
            Mod::MomentumTransfer => "momentum_transfer",
            Mod::StrandAmmoGeneration => "strand_ammo_generation",
            Mod::Absolution => "absolution",
        };

        write!(f, "{name}")
    }
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
    pub fn value(&self) -> u8 {
        match *self {
            Stat::Health(v) => v,
            Stat::Melee(v) => v,
            Stat::Grenade(v) => v,
            Stat::Super(v) => v,
            Stat::Class(v) => v,
            Stat::Weapons(v) => v,
        }
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Stat::Health(_) => "health",
            Stat::Melee(_) => "melee",
            Stat::Grenade(_) => "grenade",
            Stat::Super(_) => "super",
            Stat::Class(_) => "class",
            Stat::Weapons(_) => "weapons",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum ArtifactPerk {
    DivinersDiscount,
    ReciprocalDraw,
    RefreshThreads,
    ElementalCoalescence,
    RadiantShrapnel,
    ElementalOverdrive,
    TightlyWoven,
    RapidPrecisionRifling,
    ElementalBenevolence,
    Shieldcrush,
    TangledWeb,
    AntiBarrierScoutAndPulse,
    FeverAndChill,
    CauterizedDarkness,
    OneWithFrost,
    FrostRenewal,
    FrigidGlare,
    ThreadedBlast,
    ThreadlingProliferation,
    PackTactics,
}

impl Display for ArtifactPerk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ArtifactPerk::DivinersDiscount => "diviners_discount",
            ArtifactPerk::ReciprocalDraw => "reciprocal_draw",
            ArtifactPerk::RefreshThreads => "refresh_threads",
            ArtifactPerk::ElementalCoalescence => "elemental_coalescence",
            ArtifactPerk::RadiantShrapnel => "radiant_shrapnel",
            ArtifactPerk::ElementalOverdrive => "elemental_overdrive",
            ArtifactPerk::TightlyWoven => "tightly_woven",
            ArtifactPerk::RapidPrecisionRifling => "rapid_precision_rifling",
            ArtifactPerk::ElementalBenevolence => "elemental_benevolence",
            ArtifactPerk::Shieldcrush => "shieldcrush",
            ArtifactPerk::TangledWeb => "tangled_web",
            ArtifactPerk::AntiBarrierScoutAndPulse => "anti_barrier_scout_and_pulse",
            ArtifactPerk::FeverAndChill => "fever_and_chill",
            ArtifactPerk::CauterizedDarkness => "cauterized_darkness",
            ArtifactPerk::OneWithFrost => "one_with_frost",
            ArtifactPerk::FrostRenewal => "frost_renewal",
            ArtifactPerk::FrigidGlare => "frigid_glare",
            ArtifactPerk::ThreadedBlast => "threaded_blast",
            ArtifactPerk::ThreadlingProliferation => "threadling_proliferation",
            ArtifactPerk::PackTactics => "pack_tactics",
        };

        write!(f, "{name}")
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
    pub const fn new(author: &'a str, dim: &'a str) -> Self {
        Self {
            author,
            dim_link: dim,
            how_it_works: None,
            video: None,
        }
    }

    pub const fn video(mut self, url: &'a str) -> Self {
        self.video = Some(url);
        self
    }
}
