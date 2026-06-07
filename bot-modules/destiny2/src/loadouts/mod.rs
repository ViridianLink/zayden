use std::fmt::{Debug, Display, Write};
use std::iter;

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
    ResolvedValue,
    SeparatorSpacingSize,
};

mod arc_hunter;
mod arc_warlock;
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
use arc_warlock::ARC_WARLOCK;
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

const BUILDS: [Loadout<'_>; 11] = [
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
    ARC_WARLOCK,
];
const DUPLICATE: EmojiId = EmojiId::new(1_395_743_560_388_706_374);

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
            let name = format!("{} | {}", build.subclass.kind, build.name);
            let value = name.to_lowercase().replace([' ', '|'], "_");

            match build.class {
                DestinyClass::Warlock => {
                    warlock_builds = warlock_builds.add_string_choice(name, value);
                },
                DestinyClass::Titan => {
                    titan_builds = titan_builds.add_string_choice(name, value);
                },
                DestinyClass::Hunter => {
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
    ) -> serenity::Result<()> {
        let Some(top) = options.pop() else {
            return Ok(());
        };
        let ResolvedValue::SubCommand(options) = top.value else {
            return Ok(());
        };

        let Some(first) = options.first() else {
            return Ok(());
        };
        let ResolvedValue::String(value) = first.value else {
            return Ok(());
        };

        let Some(build) = BUILDS.iter().copied().find(|build| {
            let subclass = build.subclass.kind.to_string().to_lowercase();
            let name = build.name.to_lowercase().replace([' ', '|'], "_");
            format!("{subclass}___{name}").as_str() == value
        }) else {
            return Ok(());
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![
                            build.into_component::<Data>(ctx, parent_token).await,
                        ]),
                ),
            )
            .await?;

        Ok(())
    }
}

impl<'a> Loadout<'a> {
    #[must_use]
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

    #[must_use]
    pub const fn tags(mut self, tags: [Option<Tag>; 3]) -> Self {
        self.tags = tags;
        self
    }

    #[must_use]
    pub const fn artifact(mut self, artifact: [Option<ArtifactPerk>; 8]) -> Self {
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
            match self.subclass.kind.into_button(emoji_cache) {
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
                self.subclass.kind, self.class
            )));

        let mut details = format!("By {}", self.details.author);
        if let Some(url) = self.details.video {
            let _ = write!(details, " • [Video Guide]({url})");
        }

        let heading2 =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
                "# {}  •  {}  •  {}\n{details}",
                self.class, self.subclass.abilities.super_, self.name
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
            match self.artifact_str(emoji_cache) {
                Ok(s) => break s,
                Err(name) => emoji_cache.upload(ctx, parent_token, &name).await,
            }
        };

        let mut misc_content = format!(
            "### Stats Priority\n#{stat_prio}\n### ARTIFACT PERKS\n#{artifact}",
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
            .into_iter()
            .map(|armour| {
                Ok(CreateContainerComponent::Section(CreateSection::new(
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

                let s =
                    if value < 200 { format!("`{value}` {emoji}") } else { emoji };

                let s = if i == 0 { format!(" {s}") } else { format!(" → {s}") };

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

impl Display for Loadout<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | {}", self.subclass.kind, self.name)
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
            Self::Warlock => write!(f, "Warlock"),
            Self::Titan => write!(f, "Titan"),
            Self::Hunter => write!(f, "Hunter"),
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
            Self::All => write!(f, "All"),
            Self::PvE => write!(f, "PvE"),
            Self::PvP => write!(f, "PvP"),
        }
    }
}

impl From<Mode> for CreateButton<'_> {
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

#[derive(Clone, Copy)]
pub struct Subclass {
    kind: SubclassType,
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
    pub fn into_button<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateButton<'a>> {
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
            Self::Arc => write!(f, "Arc"),
            Self::Void => write!(f, "Void"),
            Self::Strand => write!(f, "Strand"),
            Self::Stasis => write!(f, "Stasis"),
            Self::Solar => write!(f, "Solar"),
            Self::Prismatic => write!(f, "Prismatic"),
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
    ChaosReach,
}

impl Display for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::BurningMaul => "Burning Maul",
            Self::GoldenGunMarksman => "Golden Gun: Marksman",
            Self::SongOfFlame => "Song of Flame",
            Self::Thundercrash => "Thundercrash",
            Self::GatheringStorm => "Gathering Storm",
            Self::Bladefury => "Bladefury",
            Self::NovaBombCataclysm => "Nova Bomb: Cataclysm",
            Self::Needlestorm => "Needlestorm",
            Self::ChaosReach => "Chaos Reach",
        };

        write!(f, "{name}")
    }
}

impl Debug for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::BurningMaul => "burning_maul",
            Self::GoldenGunMarksman => "golden_gun__marksman",
            Self::SongOfFlame => "song_of_flame",
            Self::Thundercrash => "thundercrash",
            Self::GatheringStorm => "gathering_storm",
            Self::Bladefury => "bladefury",
            Self::NovaBombCataclysm => "nova_bomb_cataclysm",
            Self::Needlestorm => "needlestorm",
            Self::ChaosReach => "chaos_reach",
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
            Self::RallyBarricade => "rally_barricade",
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
    BallLightning,
}

impl Display for Melee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    ArcSoul,
    IonicSentry,
}

impl Display for Aspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::RoaringFlames => "roaring_flames",
            Self::SolInvictus => "sol_invictus",
            Self::Ascension => "ascension",
            Self::GunpowderGamble => "gunpowder_gamble",
            Self::TouchOfFlame => "touch_of_flame",
            Self::Hellion => "hellion",
            Self::Knockout => "knockout",
            Self::DiamondLance => "diamond_lance",
            Self::TempestStrike => "tempest_strike",
            Self::FlowState => "flow_state",
            Self::StylishExecutioner => "stylish_executioner",
            Self::WintersShroud => "winters_shroud",
            Self::BannerOfWar => "banner_of_war",
            Self::FlechetteStorm => "flechette_storm",
            Self::ChaosAccelerant => "chaos_accelerant",
            Self::FeedTheVoid => "feed_the_void",
            Self::Weavewalk => "weavewalk",
            Self::WeaversCall => "weavers_call",
            Self::LightningSurge => "lightning_surge",
            Self::ArcSoul => "arc_soul",
            Self::IonicSentry => "ionic_sentry",
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    #[must_use]
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

        let content = if mods.is_empty() {
            format!("**{:?}**", self.name)
        } else {
            format!("**{:?}**\n#{mods}", self.name)
        };

        Ok(CreateTextDisplay::new(content))
    }

    pub fn into_section<'a>(
        self,
        emoji_cache: &EmojiCache,
    ) -> EmojiResult<CreateSectionComponent<'a>> {
        Ok(CreateSectionComponent::TextDisplay(self.into_text_display(emoji_cache)?))
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
    TechsecGloves,
    TechsecVestment,
    TwofoldCrownBoots,
    TwofoldCrownBond,
}

impl Display for ArmourName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let url = match self {
            Self::MelasPanoplia => {
                "https://www.bungie.net/common/destiny2_content/icons/8546b88189f69d88f8efa3d258f67026.jpg"
            },
            Self::WormgodCaress => {
                "https://www.bungie.net/common/destiny2_content/icons/f93fb202061de21b42138c9348359d27.jpg"
            },
            Self::BushidoHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/9879c7eda4c3bcb56712a964f57717e9.jpg"
            },
            Self::BushidoPlate => {
                "https://www.bungie.net/common/destiny2_content/icons/35c2f575bf2584e4e9729bcbb5c62a85.jpg"
            },
            Self::BushidoGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/aaab3065cf9f92898ef641da58b2585b.jpg"
            },
            Self::BushidoMark => {
                "https://www.bungie.net/common/destiny2_content/icons/9376932f07459b7a5858dfa73730c84c.jpg"
            },
            Self::BushidoCowl => {
                "https://www.bungie.net/common/destiny2_content/icons/9c38bcbbb84005d4c1bd6b9184a58571.jpg"
            },
            Self::BushidoGrips => {
                "https://www.bungie.net/common/destiny2_content/icons/8e948205999822eb4ba7933ef05ba56c.jpg"
            },
            Self::LastDisciplineVest => {
                "https://www.bungie.net/common/destiny2_content/icons/1f3f5870b6e1163d589da044c48a20ca.jpg"
            },
            Self::LastDisciplineStrides => {
                "https://www.bungie.net/common/destiny2_content/icons/db74932fddacc7a8a98844f2480e4a7f.jpg"
            },
            Self::CollectivePsycheCover => {
                "https://www.bungie.net/common/destiny2_content/icons/41157409d6cfd4da8f44f36f1f7d7e40.jpg"
            },
            Self::CollectivePsycheGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/fec9d8ed57853226cc031d6ffed9a70c.jpg"
            },
            Self::StarfireProtocol => {
                "https://www.bungie.net/common/destiny2_content/icons/707703c3e72776cbf463a2d6427f5b43.jpg"
            },
            Self::CollectivePsycheBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/8d9a8b0ba16b2d0bc9fa5ab1266ecb9b.jpg"
            },
            Self::CollectivePsycheBond => {
                "https://www.bungie.net/common/destiny2_content/icons/19e70cd67f1f361003bcdaa59952fbab.jpg"
            },
            Self::LustrousHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/67d2e115db35baf3509a7a54d2d620be.jpg"
            },
            Self::LustrousPlate => {
                "https://www.bungie.net/common/destiny2_content/icons/b82af1a81e8fdf6f3101c3ec85116387.jpg"
            },
            Self::LustrousGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/775e22c8c987b15e3834efcb35c84996.jpg"
            },
            Self::LustrousMark => {
                "https://www.bungie.net/common/destiny2_content/icons/7e2d5b6b4bfbc99b00f1447836ba6795.jpg"
            },
            Self::AnInsurmountableSkullfort => {
                "https://www.bungie.net/common/destiny2_content/icons/b734daf76fba2c835ba58ebca84c1d61.jpg"
            },
            Self::CollectivePsycheGauntlets => {
                "https://www.bungie.net/common/destiny2_content/icons/98aeaf66c0dd814cb1d72ef4b1c725bc.jpg"
            },
            Self::CollectivePsychePlate => {
                "https://www.bungie.net/common/destiny2_content/icons/edbc60a615bd223bfe4cd30c46a58d49.jpg"
            },
            Self::CollectivePsycheGreaves => {
                "https://www.bungie.net/common/destiny2_content/icons/d8a5bd616380eff7886b55cf5a496111.jpg"
            },
            Self::CollectivePsycheMark => {
                "https://www.bungie.net/common/destiny2_content/icons/845d32ecf59ca0eea8c54cf9e108eb3d.jpg"
            },
            Self::MaskOfBakris => {
                "https://www.bungie.net/common/destiny2_content/icons/c753c91b8ff629cc60e835aebc8da958.jpg"
            },
            Self::Relativism(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/e4acc5bd83081bcf82f8e7c8905b58c4.jpg"
            },
            Self::BushidoVest => {
                "https://www.bungie.net/common/destiny2_content/icons/982d331f44b50ab074c856effdf4ac23.jpg"
            },
            Self::LastDisciplineCloak => {
                "https://www.bungie.net/common/destiny2_content/icons/da32491871e833d20955b2f055d59ab6.jpg"
            },
            Self::CollectivePsycheCasque => {
                "https://www.bungie.net/common/destiny2_content/icons/2ad2c64c11a5b3f86382cfb94517a561.jpg"
            },
            Self::CollectivePsycheCuirass => {
                "https://www.bungie.net/common/destiny2_content/icons/0aa178e78bb12e1962e183b2696f9f92.jpg"
            },
            Self::CollectivePsycheSleeves => {
                "https://www.bungie.net/common/destiny2_content/icons/f64ecc6277d8a4df49813adb071e4dbb.jpg"
            },
            Self::CollectivePsycheStrides => {
                "https://www.bungie.net/common/destiny2_content/icons/7b661a41864b375de2a3d4b299cd8a99.jpg"
            },
            Self::CollectivePsycheHelm => {
                "https://www.bungie.net/common/destiny2_content/icons/eded09222a4d5bab546ad3cf04d24bf3.jpg"
            },
            Self::WishfulIgnorance => {
                "https://www.bungie.net/common/destiny2_content/icons/4a0247f3edb22758ba945e6ba341721b.jpg"
            },
            Self::GiftedConviction => {
                "https://www.bungie.net/common/destiny2_content/icons/a8f8856e51daa04775b2d510b2ca12f1.jpg"
            },
            Self::HunterHelmet => {
                "https://www.bungie.net/common/destiny2_content/icons/d2abc2257f85934b8ff763e563f02cd9.jpg"
            },
            Self::HunterArms => {
                "https://www.bungie.net/common/destiny2_content/icons/1cfe58452f5dae674b7f6d0f816e9592.jpg"
            },
            Self::HunterLegs => {
                "https://www.bungie.net/common/destiny2_content/icons/9cc3f7461305a1ece9f91f5a25d9e7a9.jpg"
            },
            Self::Cloak => {
                "https://www.bungie.net/common/destiny2_content/icons/363fd4e1311408d0f5400f6d9579cf2f.jpg"
            },
            Self::VeritysBrow => {
                "https://www.bungie.net/common/destiny2_content/icons/1eaa3f087b696caa6e8308e65883fb22.jpg"
            },
            Self::AionAdapterGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/3300af1f577f999d59651d10ee16df52.jpg"
            },
            Self::AionAdapterRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/6e431ef7eb277ca27ac4204b32cf03a1.jpg"
            },
            Self::AionAdapterBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/09a5ab08b8f9f258fe5357a67188a3c9.jpg"
            },
            Self::AionAdapterBond => {
                "https://www.bungie.net/common/destiny2_content/icons/ed84553c654e3c5a74c83efa5354ffd8.jpg"
            },
            Self::AionAdapterHood => {
                "https://www.bungie.net/common/destiny2_content/icons/fc6f5043c2e35c80fa87cf557e105cb7.jpg"
            },
            Self::AIONRenewalRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/90c55f512d646cf5100af428a194fdd0.jpg"
            },
            Self::Swarmers => {
                "https://www.bungie.net/common/destiny2_content/icons/1267deeabc5cb6863332d4ec05b5afc8.jpg"
            },
            Self::AIONRenewalBond => {
                "https://www.bungie.net/common/destiny2_content/icons/2d4242012ce9246f3289dafddfa9dd60.jpg"
            },
            Self::WarlockHood => {
                "https://www.bungie.net/common/destiny2_content/icons/1cb2285f74ece98b03e170a3f8d9abdc.jpg"
            },
            Self::WarlockGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/bfece8a540293e1ac584d894caaa7258.jpg"
            },
            Self::WarlockRobes => {
                "https://www.bungie.net/common/destiny2_content/icons/9fc0d6f0828aea5abe2f13354c6e63b5.jpg"
            },
            Self::WarlockBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/1c3ae268b2f129c252f0609fe52b8028.jpg"
            },
            Self::Solipsism(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/5d657945620203cc8a7b5ade47e6e12a.jpg"
            },
            Self::TechsecGloves => {
                "https://www.bungie.net/common/destiny2_content/icons/fe1fcf9002c3148bd933801a43613102.jpg"
            },
            Self::TechsecVestment => {
                "https://www.bungie.net/common/destiny2_content/icons/2e53661958423ed5bfd1fcdd3d2f0ec9.jpg"
            },
            Self::TwofoldCrownBoots => {
                "https://www.bungie.net/common/destiny2_content/icons/190b1833593db2263bf8318e59f1db31.jpg"
            },
            Self::TwofoldCrownBond => {
                "https://www.bungie.net/common/destiny2_content/icons/efcc8e332f9d5a3c8ef4b4d0511f7673.jpg"
            },
        };

        write!(f, "{url}")
    }
}

impl Debug for ArmourName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::MelasPanoplia => "Melas Panoplia",
            Self::WormgodCaress => "Wormgod Caress",
            Self::BushidoHelm => "Bushido Helm",
            Self::BushidoPlate => "Bushido Plate",
            Self::BushidoGreaves => "Bushido Greaves",
            Self::BushidoMark => "Bushido Mark",
            Self::BushidoCowl => "Bushido Cowl",
            Self::BushidoGrips => "Bushido Grips",
            Self::LastDisciplineVest => "Last Discipline Vest",
            Self::LastDisciplineStrides => "Last Discipline Strides",
            Self::CollectivePsycheCover => "Collective Psyche Cover",
            Self::CollectivePsycheGloves => "Collective Psyche Gloves",
            Self::StarfireProtocol => "Starfire Protocol",
            Self::CollectivePsycheBoots => "Collective Psyche Boots",
            Self::CollectivePsycheBond => "Collective PsycheBond",
            Self::LustrousHelm => "Lustrous Helm",
            Self::LustrousPlate => "Lustrous Plate",
            Self::LustrousGreaves => "Lustrous Greaves",
            Self::LustrousMark => "Lustrous Mark",
            Self::AnInsurmountableSkullfort => "An Insurmountable Skullfort",
            Self::CollectivePsycheGauntlets => "Collective Psyche Gauntlets",
            Self::CollectivePsychePlate => "Collective Psyche Plate",
            Self::CollectivePsycheGreaves => "Collective Psyche Greaves",
            Self::CollectivePsycheMark => "Collective Psyche Mark",
            Self::MaskOfBakris => "Mask of Bakris",
            Self::Relativism(perks) => {
                &format!("Relativism ({} + {})", perks.0, perks.1)
            },
            Self::BushidoVest => "Bushido Vest",
            Self::LastDisciplineCloak => "Last Discipline Cloak",
            Self::CollectivePsycheCasque => "Collective Psyche Casque",
            Self::CollectivePsycheCuirass => "Collective Psyche Cuirass",
            Self::CollectivePsycheSleeves => "Collective Psyche Sleeves",
            Self::CollectivePsycheStrides => "Collective Psyche Strides",
            Self::CollectivePsycheHelm => "Collective Psyche Helm",
            Self::WishfulIgnorance => "Wishful Ignorance",
            Self::GiftedConviction => "Gifted Conviction",
            Self::HunterHelmet => "Any Helment",
            Self::HunterArms => "Any Arms",
            Self::HunterLegs => "Any Legs",
            Self::Cloak => "Any Cloak",
            Self::AionAdapterGloves => "Aion Adapter Gloves",
            Self::AionAdapterRobes => "Aion Adapter Robes",
            Self::AionAdapterBoots => "Aion Adapter Boots",
            Self::AionAdapterBond => "Aion Adapter Bond",
            Self::VeritysBrow => "Verity's Brow",
            Self::AionAdapterHood => "AION Adapter Hood",
            Self::AIONRenewalRobes => "AION Renewal Robes",
            Self::Swarmers => "Swarmers",
            Self::AIONRenewalBond => "AION Renewal Bond",
            Self::WarlockHood => "Any Hood",
            Self::WarlockGloves => "Any Gloves",
            Self::WarlockRobes => "Any Robe",
            Self::WarlockBoots => "Any Boots",
            Self::Solipsism(perks) => {
                &format!("Solipsism ({} + {})", perks.0, perks.1)
            },
            Self::TechsecGloves => "Techsec Gloves",
            Self::TechsecVestment => "Techsec Vestment",
            Self::TwofoldCrownBoots => "Twofold Crown Boots",
            Self::TwofoldCrownBond => "Twofold Crown Bond",
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
    BolsteringDetonation,
    HarmonicAmmoGeneration,
}

impl Display for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Empty => "empty_mod",
            Self::HandsOn => "hands_on",
            Self::SpecialAmmoFinder => "special_ammo_finder",
            Self::HarmonicSiphon => "harmonic_siphon",
            Self::MeleeFont => "melee_font",
            Self::HeavyHanded => "heavy_handed",
            Self::StacksOnStacks => "stacks_on_stacks",
            Self::KineticScavenger => "kinetic_scavenger",
            Self::TimeDilation => "time_dilation",
            Self::Reaper => "reaper",
            Self::SpecialFinisher => "special_finisher",
            Self::AshesToAssets => "ashes_to_assets",
            Self::SuperFont => "super_font",
            Self::VoidSiphon => "void_siphon",
            Self::Firepower => "firepower",
            Self::GrenadeFont => "grenade_font",
            Self::FocusingStrike => "focusing_strike",
            Self::Recuperation => "recuperation",
            Self::Invigoration => "invigoration",
            Self::ClassFont => "class_font",
            Self::PowerfulAttraction => "powerful_attraction",
            Self::Innervation => "innervation",
            Self::StrandScavenger => "strand_scavenger",
            Self::Distribution => "distribution",
            Self::StasisSiphon => "stasis_siphon",
            Self::ImpactInduction => "impact_induction",
            Self::HarmonicLoader => "harmonic_loader",
            Self::ArcWeaponSurge => "arc_weapon_surge",
            Self::StasisWeaponSurge => "stasis_weapon_surge",
            Self::StrandSiphon => "strand_siphon",
            Self::Outreach => "outreach",
            Self::KineticSiphon => "kinetic_siphon",
            Self::VoidAmmoGeneration => "void_ammo_generation",
            Self::WeaponsFont => "weapons_font",
            Self::VoidScavenger => "void_scavenger",
            Self::HarmonicScavenger => "harmonic_scavenger",
            Self::MomentumTransfer => "momentum_transfer",
            Self::StrandAmmoGeneration => "strand_ammo_generation",
            Self::Absolution => "absolution",
            Self::BolsteringDetonation => "bolstering_detonation",
            Self::HarmonicAmmoGeneration => "harmonic_ammo_generation",
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            Self::DivinersDiscount => "diviners_discount",
            Self::ReciprocalDraw => "reciprocal_draw",
            Self::RefreshThreads => "refresh_threads",
            Self::ElementalCoalescence => "elemental_coalescence",
            Self::RadiantShrapnel => "radiant_shrapnel",
            Self::ElementalOverdrive => "elemental_overdrive",
            Self::TightlyWoven => "tightly_woven",
            Self::RapidPrecisionRifling => "rapid_precision_rifling",
            Self::ElementalBenevolence => "elemental_benevolence",
            Self::Shieldcrush => "shieldcrush",
            Self::TangledWeb => "tangled_web",
            Self::AntiBarrierScoutAndPulse => "anti_barrier_scout_and_pulse",
            Self::FeverAndChill => "fever_and_chill",
            Self::CauterizedDarkness => "cauterized_darkness",
            Self::OneWithFrost => "one_with_frost",
            Self::FrostRenewal => "frost_renewal",
            Self::FrigidGlare => "frigid_glare",
            Self::ThreadedBlast => "threaded_blast",
            Self::ThreadlingProliferation => "threadling_proliferation",
            Self::PackTactics => "pack_tactics",
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
