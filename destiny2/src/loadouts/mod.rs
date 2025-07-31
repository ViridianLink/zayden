use std::fmt::{Debug, Display};

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateComponent, CreateContainer,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateSection,
    CreateSectionAccessory, CreateSectionComponent, CreateSeparator, CreateTextDisplay,
    CreateThumbnail, CreateUnfurledMediaItem, EmojiId, Http, MessageFlags, ResolvedOption,
    ResolvedValue, Spacing,
};

mod prismatic_hunter;
mod solar_titan;
mod solar_warlock;
use prismatic_hunter::PRISMATIC_HUNTER;
use solar_titan::SOLAR_TITAN;
use solar_warlock::SOLAR_WARLOCK;

pub mod weapons;
pub use weapons::{Perk, Weapon};

const BUILDS: [Loadout; 3] = [PRISMATIC_HUNTER, SOLAR_TITAN, SOLAR_WARLOCK];
const DUPLICATE: EmojiId = EmojiId::new(1395743560388706374);

#[derive(Clone, Copy)]
pub struct Loadout<'a> {
    name: &'a str,
    class: DestinyClass,
    mode: Mode,
    tags: [Option<Tag>; 3],
    subclass: Subclass,
    gear: Gear<'a>,
    artifact: [Option<ArtifactPerk>; 7],
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

    pub async fn run(
        http: &Http,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
    ) {
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
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![build.into()]),
                ),
            )
            .await
            .unwrap()
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
            artifact: [None; 7],
            details,
        }
    }

    pub const fn tags(mut self, tags: [Option<Tag>; 3]) -> Self {
        self.tags = tags;
        self
    }

    pub const fn artifact(mut self, artifact: [Option<ArtifactPerk>; 7]) -> Self {
        self.artifact = artifact;
        self
    }
}

impl<'a> Display for Loadout<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} | {}", self.subclass.subclass, self.name)
    }
}

impl<'a> From<Loadout<'a>> for CreateComponent<'a> {
    fn from(value: Loadout<'a>) -> Self {
        let tags = CreateComponent::ActionRow(CreateActionRow::buttons(
            [CreateButton::from(value.subclass.subclass)]
                .into_iter()
                .chain([CreateButton::from(value.mode)])
                .chain(value.tags.into_iter().flatten().map(CreateButton::from))
                .collect::<Vec<_>>(),
        ));

        let heading1 = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "-# {} {} Build",
            value.subclass.subclass, value.class
        )));

        let mut details = format!("By {}", value.details.author);
        if let Some(url) = value.details.video {
            details.push_str(&format!(" • [Video Guide]({url})"));
        }

        let heading2 = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "# {}  •  {:?}  •  {}\n{details}",
            value.class, value.subclass.abilities.super_, value.name
        )));

        let line_sep = CreateComponent::Separator(CreateSeparator::new(true));

        let dim_link = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            CreateButton::new_link(value.details.dim_link)
                .label("COPY DIM LINK")
                .emoji(DUPLICATE),
        ]));

        let subclass_heading = CreateComponent::TextDisplay(CreateTextDisplay::new(
            "### SUBCLASS\nSuper       Abilities                                       Aspects",
        ));

        let aspects = value.subclass.aspects.map(|a| a.to_string()).join(" ");

        let subclass = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "# {}    {} {} {} {}    {aspects}\n\nFragments",
            value.subclass.abilities.super_,
            value.subclass.abilities.class,
            value.subclass.abilities.jump,
            value.subclass.abilities.melee,
            value.subclass.abilities.grenade
        )));

        let fragments = CreateComponent::TextDisplay(CreateTextDisplay::new(format!(
            "#{}",
            value
                .subclass
                .fragments
                .into_iter()
                .flatten()
                .map(|f| format!(" {f}"))
                .collect::<String>()
        )));

        let gear_and_mods_heading =
            CreateComponent::TextDisplay(CreateTextDisplay::new("### GEAR AND MODS"));

        let weapons = value.gear.weapons.into_iter().flatten().map(|weapon| {
            CreateComponent::Section(CreateSection::new(
                vec![weapon.into()],
                CreateSectionAccessory::Thumbnail(weapon.into()),
            ))
        });

        let armour = value.gear.armour.map(|armour| {
            CreateComponent::Section(CreateSection::new(
                vec![armour.into()],
                CreateSectionAccessory::Thumbnail(armour.into()),
            ))
        });

        let mut misc_content = format!(
            "### Stats Priority\n#{}\n### ARTIFACT PERKS\n#{}",
            value
                .gear
                .stats_priority
                .into_iter()
                .map(|f| format!(" {f}"))
                .collect::<String>(),
            value
                .artifact
                .into_iter()
                .flatten()
                .map(|f| format!(" {f}"))
                .collect::<String>()
        );

        if let Some(how_it_works) = value.details.how_it_works {
            misc_content.push_str("\n### HOW IT WORKS\n# ");
            misc_content.push_str(how_it_works);
        }

        let misc = CreateComponent::TextDisplay(CreateTextDisplay::new(misc_content));

        let mut components = vec![
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
        ];

        components.extend(weapons);
        components.push(CreateComponent::Separator(
            CreateSeparator::new(false).spacing(Spacing::Large),
        ));
        components.extend(armour);
        components.push(misc);

        CreateComponent::Container(CreateContainer::new(components))
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
        match self {
            Tag::EasyToPlay => todo!(),
            Tag::BossDamage => todo!(),
            Tag::AdClear => todo!(),
            Tag::HighSurvivability => write!(f, "High Survivability"),
            Tag::Support => todo!(),
            Tag::AntiChampion => todo!(),
            Tag::CasualPvP => todo!(),
            Tag::CompetitivePvp => todo!(),
            Tag::Raids => todo!(),
            Tag::Dungeons => todo!(),
            Tag::MasterContent => todo!(),
            Tag::GrandmasterNightfall => todo!(),
            Tag::Solo => todo!(),
            Tag::SuperFocused => todo!(),
            Tag::AbilityFocused => write!(f, "Ability Focused"),
            Tag::WeaponFocused => todo!(),
            Tag::HighDamage => todo!(),
            Tag::EndGame => todo!(),
            Tag::CrowdControl => todo!(),
        }
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

impl From<SubclassType> for EmojiId {
    fn from(value: SubclassType) -> Self {
        match value {
            SubclassType::Arc => todo!(),
            SubclassType::Void => EmojiId::new(1396107597123293254),
            SubclassType::Strand => todo!(),
            SubclassType::Stasis => todo!(),
            SubclassType::Solar => EmojiId::new(1395737098220212345),
            SubclassType::Prismatic => EmojiId::new(1396109157312233483),
        }
    }
}

impl<'a> From<SubclassType> for CreateButton<'a> {
    fn from(value: SubclassType) -> Self {
        CreateButton::new(format!("{value}"))
            .label(format!("{value}"))
            .emoji(EmojiId::from(value))
            .style(ButtonStyle::Secondary)
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
}

impl Debug for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::BurningMaul => "Burning Maul",
            Self::GoldenGunMarksman => "Golden Gun: Marksman",
            Self::SongOfFlame => "Song of Flame",
        };

        write!(f, "{name}")
    }
}

impl Display for Super {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = format!("{self:?}");

        write!(
            f,
            "<:{}:{}>",
            name.to_lowercase().replace([' ', ':'], "_"),
            EmojiId::from(*self)
        )
    }
}

impl From<Super> for EmojiId {
    fn from(value: Super) -> Self {
        match value {
            Super::BurningMaul => EmojiId::new(1395756177563979869),
            Super::GoldenGunMarksman => EmojiId::new(1396093970161078272),
            Super::SongOfFlame => EmojiId::new(1399525852567572610),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ClassAbility {
    RallyBarricade,
    MarksmansDodge,
    PhoenixDive,
}

impl Display for ClassAbility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::RallyBarricade => "rally_barricade",
            Self::MarksmansDodge => "marksmans_dodge",
            Self::PhoenixDive => "phoenix_dive",
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<ClassAbility> for EmojiId {
    fn from(value: ClassAbility) -> Self {
        match value {
            ClassAbility::RallyBarricade => EmojiId::new(1395888733152219256),
            ClassAbility::MarksmansDodge => EmojiId::new(1396094606575140884),
            ClassAbility::PhoenixDive => EmojiId::new(1399526158252642496),
        }
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

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Jump> for EmojiId {
    fn from(value: Jump) -> Self {
        match value {
            Jump::CatapultLift => EmojiId::new(1395888809228369921),
            Jump::Triple => EmojiId::new(1396094896104013884),
            Jump::BurstGlide => EmojiId::new(1399526487933190154),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Melee {
    ThrowingHammer,
    ThreadedSpike,
    IncineratorSnap,
}

impl Display for Melee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::ThrowingHammer => "throwing_hammer",
            Self::ThreadedSpike => "threaded_spike",
            Self::IncineratorSnap => "incinerator_snap",
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Melee> for EmojiId {
    fn from(value: Melee) -> Self {
        match value {
            Melee::ThrowingHammer => EmojiId::new(1395889006280970260),
            Melee::ThreadedSpike => EmojiId::new(1396095199934939166),
            Melee::IncineratorSnap => EmojiId::new(1399527592532377670),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Grenade {
    Healing,
    Grapple,
    Fusion,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Healing => "healing_grenade",
            Self::Grapple => "grapple",
            Self::Fusion => "fusion_grenade",
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Grenade> for EmojiId {
    fn from(value: Grenade) -> Self {
        match value {
            Grenade::Healing => EmojiId::new(1395889096768880691),
            Grenade::Grapple => EmojiId::new(1396095515027964057),
            Grenade::Fusion => EmojiId::new(1399527741656924192),
        }
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
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Aspect> for EmojiId {
    fn from(value: Aspect) -> Self {
        match value {
            Aspect::RoaringFlames => EmojiId::new(1395889677868732597),
            Aspect::SolInvictus => EmojiId::new(1395889685271806013),
            Aspect::Ascension => EmojiId::new(1396095931849375867),
            Aspect::GunpowderGamble => EmojiId::new(1396095943257886761),
            Aspect::TouchOfFlame => EmojiId::new(1399528027435569344),
            Aspect::Hellion => EmojiId::new(1399528095634948208),
        }
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
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Fragment> for EmojiId {
    fn from(value: Fragment) -> Self {
        match value {
            Fragment::EmberOfAshes => EmojiId::new(1395890217734504508),
            Fragment::EmberOfEmpyrean => EmojiId::new(1395890268162625696),
            Fragment::EmberOfSearing => EmojiId::new(1395890300878323853),
            Fragment::EmberOfTorches => EmojiId::new(1395890327482667058),
            Fragment::EmberOfMercy => EmojiId::new(1399528431661744379),
            Fragment::FacetOfHope => EmojiId::new(1396096661578842173),
            Fragment::FacetOfProtection => EmojiId::new(1396096705711046766),
            Fragment::FacetOfPurpose => EmojiId::new(1396096749038211184),
            Fragment::FacetOfDawn => EmojiId::new(1396096787810488390),
            Fragment::FacetOfBlessing => EmojiId::new(1396096821343948891),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Gear<'a> {
    weapons: [Option<Weapon<'a>>; 3],
    armour: [Armour<'a>; 5],
    stats_priority: [Stat; 6],
}

#[derive(Clone, Copy)]
pub struct Armour<'a> {
    name: &'a str,
    mods: [Mod; 3],
}

impl<'a> Armour<'a> {
    pub const fn new(name: &'a str, mods: [Mod; 3]) -> Self {
        Self { name, mods }
    }
}

impl<'a> From<Armour<'a>> for CreateSectionComponent<'a> {
    fn from(value: Armour<'a>) -> Self {
        CreateSectionComponent::TextDisplay(value.into())
    }
}

impl<'a> From<Armour<'a>> for CreateTextDisplay<'a> {
    fn from(value: Armour<'a>) -> Self {
        let mods = value
            .mods
            .into_iter()
            .map(|m| format!(" {m}"))
            .collect::<String>();

        let content = if !mods.is_empty() {
            format!("**{}**\n#{mods}", value.name)
        } else {
            format!("**{}**", value.name)
        };

        CreateTextDisplay::new(content)
    }
}

impl<'a> From<Armour<'a>> for CreateThumbnail<'a> {
    fn from(value: Armour<'a>) -> Self {
        CreateThumbnail::new(value.into())
    }
}

impl<'a> From<Armour<'a>> for CreateUnfurledMediaItem<'a> {
    fn from(value: Armour) -> Self {
        let url = match value.name {
            "Bushido Helm" => {
                "https://www.bungie.net/common/destiny2_content/icons/9879c7eda4c3bcb56712a964f57717e9.jpg"
            }
            "Melas Panoplia" => {
                "https://www.bungie.net/common/destiny2_content/icons/8546b88189f69d88f8efa3d258f67026.jpg"
            }
            "Bushido Plate" => {
                "https://www.bungie.net/common/destiny2_content/icons/35c2f575bf2584e4e9729bcbb5c62a85.jpg"
            }
            "Bushido Greaves" => {
                "https://www.bungie.net/common/destiny2_content/icons/aaab3065cf9f92898ef641da58b2585b.jpg"
            }
            "Bushido Mark" => {
                "https://www.bungie.net/common/destiny2_content/icons/9376932f07459b7a5858dfa73730c84c.jpg"
            }
            "Bushido Cowl" => {
                "https://www.bungie.net/common/destiny2_content/icons/9c38bcbbb84005d4c1bd6b9184a58571.jpg"
            }
            "Bushido Grips" => {
                "https://www.bungie.net/common/destiny2_content/icons/8e948205999822eb4ba7933ef05ba56c.jpg"
            }
            "Last Discipline Vest" => {
                "https://www.bungie.net/common/destiny2_content/icons/1f3f5870b6e1163d589da044c48a20ca.jpg"
            }
            "Last Discipline Strides" => {
                "https://www.bungie.net/common/destiny2_content/icons/db74932fddacc7a8a98844f2480e4a7f.jpg"
            }
            "Collective Psyche Cover" => {
                "https://www.bungie.net/common/destiny2_content/icons/41157409d6cfd4da8f44f36f1f7d7e40.jpg"
            }
            "Collective Psyche Gloves" => {
                "https://www.bungie.net/common/destiny2_content/icons/fec9d8ed57853226cc031d6ffed9a70c.jpg"
            }
            "Starfire Protocol" => {
                "https://www.bungie.net/common/destiny2_content/icons/707703c3e72776cbf463a2d6427f5b43.jpg"
            }
            "Collective Psyche Boots" => {
                "https://www.bungie.net/common/destiny2_content/icons/8d9a8b0ba16b2d0bc9fa5ab1266ecb9b.jpg"
            }
            "Collective Psyche Bond" => {
                "https://www.bungie.net/common/destiny2_content/icons/19e70cd67f1f361003bcdaa59952fbab.jpg"
            }
            name if name.starts_with("Relativism") => {
                "https://www.bungie.net/common/destiny2_content/icons/e4acc5bd83081bcf82f8e7c8905b58c4.jpg"
            }
            name => unimplemented!("Image URL for '{name}' not implemented"),
        };

        CreateUnfurledMediaItem::new(url)
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
}

impl Display for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Empty => "empty",
            Self::HandsOn => "hands_on",
            Self::SpecialAmmoFinder => "special_ammo_finder",
            Self::HarmonicSiphon => "harmonic_siphone",
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
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Mod> for EmojiId {
    fn from(value: Mod) -> Self {
        match value {
            Mod::Empty => EmojiId::new(1395896423953862778),
            Mod::HandsOn => EmojiId::new(1395894177883095070),
            Mod::SpecialAmmoFinder => EmojiId::new(1395894196006551552),
            Mod::HarmonicSiphon => EmojiId::new(1395894270308647053),
            Mod::MeleeFont => EmojiId::new(1395894286431686718),
            Mod::HeavyHanded => EmojiId::new(1395894261072920637),
            Mod::StacksOnStacks => EmojiId::new(1395894226012864523),
            Mod::KineticScavenger => EmojiId::new(1395894235072430223),
            Mod::TimeDilation => EmojiId::new(1395894187316088993),
            Mod::Reaper => EmojiId::new(1395894278466834635),
            Mod::SpecialFinisher => EmojiId::new(1395894202914832384),
            Mod::AshesToAssets => EmojiId::new(1396097791998038067),
            Mod::SuperFont => EmojiId::new(1396097835338043502),
            Mod::VoidSiphon => EmojiId::new(1396097870834307193),
            Mod::Firepower => EmojiId::new(1396097935363801118),
            Mod::GrenadeFont => EmojiId::new(1396097966431010917),
            Mod::FocusingStrike => EmojiId::new(1396098012509376553),
            Mod::Recuperation => EmojiId::new(1396098046588227696),
            Mod::Invigoration => EmojiId::new(1396098080335466547),
            Mod::ClassFont => EmojiId::new(1396098129534652540),
            Mod::PowerfulAttraction => EmojiId::new(1396098159553286264),
            Mod::Innervation => EmojiId::new(1399528773149528065),
            Mod::StrandScavenger => EmojiId::new(1399528781030625340),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Stat {
    Health,
    Melee,
    Grenade,
    Super,
    Class,
    Weapons,
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Stat::Health => "health",
            Stat::Melee => "melee",
            Stat::Grenade => "grenade",
            Stat::Super => "super",
            Stat::Class => "class",
            Stat::Weapons => "weapons",
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<Stat> for EmojiId {
    fn from(value: Stat) -> Self {
        match value {
            Stat::Health => EmojiId::new(1396955669063536751),
            Stat::Melee => EmojiId::new(1396955747480375326),
            Stat::Grenade => EmojiId::new(1396955787510939738),
            Stat::Super => EmojiId::new(1396955844544954378),
            Stat::Class => EmojiId::new(1396955885418447050),
            Stat::Weapons => EmojiId::new(1396955919769800844),
        }
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
        };

        write!(f, "<:{name}:{}>", EmojiId::from(*self))
    }
}

impl From<ArtifactPerk> for EmojiId {
    fn from(value: ArtifactPerk) -> Self {
        match value {
            ArtifactPerk::DivinersDiscount => EmojiId::new(1395895452720955654),
            ArtifactPerk::ReciprocalDraw => EmojiId::new(1395895519993139230),
            ArtifactPerk::RefreshThreads => EmojiId::new(1395895483943223356),
            ArtifactPerk::ElementalCoalescence => EmojiId::new(1395895708934209586),
            ArtifactPerk::RadiantShrapnel => EmojiId::new(1395895735681286226),
            ArtifactPerk::ElementalOverdrive => EmojiId::new(1395895790123094212),
            ArtifactPerk::TightlyWoven => EmojiId::new(1396098977904066560),
            ArtifactPerk::RapidPrecisionRifling => EmojiId::new(1396099003656966174),
            ArtifactPerk::ElementalBenevolence => EmojiId::new(1396098986917629973),
            ArtifactPerk::Shieldcrush => EmojiId::new(1396099019796910180),
            ArtifactPerk::TangledWeb => EmojiId::new(1396098996459802704),
            ArtifactPerk::AntiBarrierScoutAndPulse => EmojiId::new(1396099011819339848),
            ArtifactPerk::FeverAndChill => EmojiId::new(1399529292609753248),
            ArtifactPerk::CauterizedDarkness => EmojiId::new(1399530731918725190),
        }
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
