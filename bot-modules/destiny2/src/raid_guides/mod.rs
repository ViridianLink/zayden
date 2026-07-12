mod desert_perpetual;
pub mod last_wish;
pub mod weapons;

use desert_perpetual::DESERT_PERPETUAL;
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateActionRow,
    CreateCommandOption,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    CreateSeparator,
    CreateTextDisplay,
    Http,
    MessageFlags,
};
use tracing::debug;
pub use weapons::Weapon;

#[derive(Clone, Copy)]
pub struct EncounterGuide<'a> {
    raid: &'a str,
    encounter: &'a str,
    video: Option<&'a str>,
    video_timestamp: Option<u8>,
    guide: &'a str,
    weapons: [Option<Weapon>; 2],
    armour: [Option<&'a str>; 3],
}

impl<'a> EncounterGuide<'a> {
    const fn new(encounter: &'a str) -> Self {
        Self {
            raid: "",
            encounter,
            video: None,
            video_timestamp: None,
            guide: "",
            weapons: [None; 2],
            armour: [None; 3],
        }
    }

    const fn video_timestamp(mut self, timestamp: u8) -> Self {
        self.video_timestamp = Some(timestamp);
        self
    }

    const fn guide(mut self, s: &'a str) -> Self {
        self.guide = s;
        self
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "index is loop-bounded by slice length"
    )]
    #[expect(
        clippy::panic,
        reason = "build-time invariant: called only with valid slot counts"
    )]
    const fn add_weapon(mut self, weapon: Weapon) -> Self {
        let mut i = 0;

        while i < self.weapons.len() {
            if self.weapons[i].is_none() {
                self.weapons[i] = Some(weapon);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "index is loop-bounded by slice length"
    )]
    #[expect(
        clippy::panic,
        reason = "build-time invariant: called only with valid slot counts"
    )]
    const fn add_armour(mut self, armour: &'a str) -> Self {
        let mut i = 0;

        while i < self.armour.len() {
            if self.armour[i].is_none() {
                self.armour[i] = Some(armour);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }
}

impl<'a> From<EncounterGuide<'a>> for CreateComponent<'a> {
    fn from(value: EncounterGuide<'a>) -> Self {
        let content = match (value.video, value.video_timestamp) {
            (Some(video), Some(timestamp)) => {
                format!(
                    "# {}\n## [{}]({video}&t={timestamp}s)",
                    value.raid, value.encounter
                )
            },
            (Some(video), _) => {
                format!("# {}\n## [{}]({video})", value.raid, value.encounter)
            },
            _ => format!("# {}\n## {}", value.raid, value.encounter),
        };

        let top_text =
            CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content));
        let seperator = CreateContainerComponent::Separator(
            CreateSeparator::new().divider(true),
        );
        let guide = CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
            value.guide,
        ));

        let weapons_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new("__Weapons:__"),
        );

        let weapons = value
            .weapons
            .iter()
            .flatten()
            .map(|&weapon| {
                let mut s = weapon.to_string();
                s.push(' ');
                s
            })
            .collect::<String>();

        let weapons = CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
            format!("# {weapons}"),
        ));

        let armour_heading = CreateContainerComponent::TextDisplay(
            CreateTextDisplay::new("__Armour:__"),
        );

        let armour = value
            .armour
            .iter()
            .flatten()
            .map(|&armour| {
                let mut s = armour.to_string();
                s.push(' ');
                s
            })
            .collect::<String>();

        let armour = CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
            format!("Recommended Armour: {armour}"),
        ));

        CreateComponent::Container(CreateContainer::new(vec![
            top_text,
            seperator.clone(),
            guide,
            seperator,
            weapons_heading,
            weapons,
            armour_heading,
            armour,
        ]))
    }
}

pub struct RaidGuide<'a, const ENCOUNTERS: usize> {
    raid: &'a str,
    video: Option<&'a str>,
    encounters: [Option<EncounterGuide<'a>>; ENCOUNTERS],
}

impl<'a, const E: usize> RaidGuide<'a, E> {
    const fn new(raid: &'a str) -> Self {
        Self { raid, video: None, encounters: [None; E] }
    }

    const fn video(mut self, url: &'a str) -> Self {
        self.video = Some(url);
        self
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "index is loop-bounded by slice length"
    )]
    #[expect(
        clippy::panic,
        reason = "build-time invariant: called only with valid encounter counts"
    )]
    const fn add_encounter(mut self, mut encounter: EncounterGuide<'a>) -> Self {
        let mut i = 0;

        while i < self.encounters.len() {
            if self.encounters[i].is_none() {
                encounter.raid = self.raid;
                encounter.video = self.video;

                self.encounters[i] = Some(encounter);
                return self;
            }
            i += 1;
        }

        panic!("Encounter list is full");
    }
}

impl<const E: usize> RaidGuide<'_, E> {
    pub fn register<'a>() -> CreateCommandOption<'a> {
        CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "raidguide",
            "Raid Guides",
        )
    }

    pub async fn run(
        http: &Http,
        interaction: &CommandInteraction,
    ) -> serenity::Result<()> {
        let page_row = CreateComponent::ActionRow(CreateActionRow::SelectMenu(
            CreateSelectMenu::new("guide_page", CreateSelectMenuKind::String {
                options: vec![
                    CreateSelectMenuOption::new("Kalli (Legit)", "kalli_legit"),
                    CreateSelectMenuOption::new(
                        "Kalli (Trapping)",
                        "kalli_trapping",
                    ),
                ]
                .into(),
            })
            .placeholder("Select encounter"),
        ));

        let Some(Some(encounter)) = DESERT_PERPETUAL.encounters.first() else {
            debug!("desert perpetual raid guide has no encounters populated");
            return Ok(());
        };

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![(*encounter).into(), page_row]),
                ),
            )
            .await?;

        Ok(())
    }
}
