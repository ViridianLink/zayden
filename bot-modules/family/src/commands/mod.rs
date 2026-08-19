mod adopt;
mod block;
mod divorce;
mod information;
mod marry;
mod moderation;
mod tree;

use std::collections::HashMap;

pub use information::collect_sibling_ids;
use serenity::all::{
    ButtonStyle,
    CommandOptionType,
    CreateActionRow,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateComponent,
    ResolvedValue,
    User,
};
pub use tree::TreeImage;
use zayden_core::{InvocationCtx, optional_option, parse_options, parse_subcommand};

use crate::{FamilyError, Result};

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("family")
            .description("Marriage, adoption, relatives, and family trees")
            .add_option(marry::register())
            .add_option(divorce::register())
            .add_option(adopt::register())
            .add_option(tree::register())
            .add_option(information::relationship::register())
            .add_option(information::children::register())
            .add_option(information::parents::register())
            .add_option(information::partner::register())
            .add_option(information::siblings::register())
            .add_option(block::register_block())
            .add_option(block::register_unblock())
            .add_option(moderation::register())
    }

    pub async fn run(cx: &InvocationCtx<'_>) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())?;
        let options = parse_options(sub_options);

        match name {
            "marry" => marry::run(cx, options).await,
            "divorce" => divorce::run(cx, options).await,
            "adopt" => adopt::run(cx, options).await,
            "tree" => tree::run(cx, options).await,
            "relationship" => information::relationship::run(cx, options).await,
            "children" => information::children::run(cx, options).await,
            "parents" => information::parents::run(cx, options).await,
            "partner" => information::partner::run(cx, options).await,
            "siblings" => information::siblings::run(cx, options).await,
            "block" => block::block(cx, options).await,
            "unblock" => block::unblock(cx, options).await,
            "reset" => moderation::run(cx).await,
            _ => Err(FamilyError::UnknownSubcommand(name.to_string())),
        }
    }
}

fn user_option(
    description: &'static str,
    required: bool,
) -> CreateCommandOption<'static> {
    CreateCommandOption::new(CommandOptionType::User, "user", description)
        .required(required)
}

fn required_user<'a>(
    options: &mut HashMap<&str, ResolvedValue<'a>>,
    name: &str,
) -> Result<&'a User> {
    optional_option(options, name).ok_or(FamilyError::InvalidUserId)
}

fn proposal_buttons<'a>(
    accept_id: &'a str,
    decline_id: &'a str,
) -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new(accept_id).label("Accept").style(ButtonStyle::Success),
        CreateButton::new(decline_id).label("Decline").style(ButtonStyle::Danger),
    ]))
}
