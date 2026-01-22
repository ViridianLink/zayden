use std::collections::HashMap;

use async_trait::async_trait;
use serenity::all::{
    ActionRow, ActionRowComponent, AutocompleteOption, CommandInteraction,
    Component as SerenityComponent, ComponentInteraction, Context, Message, ModalInteraction,
    ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};

mod application_command;
pub use application_command::ApplicationCommand;

pub mod cache;
pub use cache::{EmojiCache, EmojiCacheData, EmojiResult, GuildMembersCache};

mod modals;
pub use modals::parse_components;

pub mod cron;
pub mod templates;
pub use cron::{ActionFn, CronJob, CronJobData};

mod error;
pub use error::Error;

pub mod events;
pub mod format_num;
pub use format_num::FormatNum;

#[async_trait]
pub trait Autocomplete<E: std::error::Error, Db: Database> {
    async fn autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait Component<E: std::error::Error, Db: Database> {
    async fn run(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait Modal<E: std::error::Error, Db: Database> {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &ModalInteraction,
        components: &[ActionRow],
        pool: &Pool<Db>,
    ) -> Result<(), E>;
}

#[async_trait]
pub trait MessageCommand<E: std::error::Error, Db: Database> {
    async fn run(ctx: &Context, message: &Message, pool: &Pool<Db>) -> Result<(), E>;
}

pub fn parse_options<'a>(
    options: impl IntoIterator<Item = ResolvedOption<'a>>,
) -> HashMap<&'a str, ResolvedValue<'a>> {
    options
        .into_iter()
        .map(|option| (option.name, option.value))
        .collect()
}

pub fn parse_options_ref<'a>(
    options: impl IntoIterator<Item = &'a ResolvedOption<'a>>,
) -> HashMap<&'a str, &'a ResolvedValue<'a>> {
    options
        .into_iter()
        .map(|option| (option.name, &option.value))
        .collect()
}

pub fn get_option_str(options: &[ResolvedOption<'_>]) -> String {
    let mut s = String::new();

    for option in options {
        s.push(' ');
        s.push_str(option.name);

        if !matches!(
            option.value,
            ResolvedValue::SubCommandGroup(_) | ResolvedValue::SubCommand(_)
        ) {
            s.push_str(": ");
        }

        match &option.value {
            ResolvedValue::SubCommandGroup(sub_options) => {
                s.push_str(&get_option_str(sub_options));
            }
            ResolvedValue::SubCommand(sub_options) => {
                s.push_str(&get_option_str(sub_options));
            }
            ResolvedValue::User(user, _) => {
                s.push_str(&format!("User({{id: {}, name: {}}})", user.id, user.name))
            }
            _ => s.push_str(&format!("{:?}", option.value)),
        }
    }

    s
}

pub fn parse_components(components: &[SerenityComponent]) -> HashMap<&str, &str> {
    components
        .iter()
        .filter_map(|component| match component {
            SerenityComponent::ActionRow(action_row) => {
                // action_row
                //     .components
                //     .iter()
                //     .filter_map(|component| todo!("Parse select menu"));

                todo!("Parse select menu")
            }
            SerenityComponent::Section(section) => todo!("Parse TextDisplay"),
            SerenityComponent::MediaGallery(media_gallery) => todo!("Parse MediaGallery"),
            SerenityComponent::File(file_component) => todo!("Parse FileComponent"),
            SerenityComponent::Label(label) => todo!("Parse Label"),
            _ => None,
        })
        .collect()
}
