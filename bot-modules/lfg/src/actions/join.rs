use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    CreateEmbed,
    GenericInteractionChannel,
    Http,
    ResolvedValue,
    ThreadId,
    UserId,
};
use sqlx::PgPool;
use zayden_core::{parse_options, parse_subcommand};

use crate::templates::DefaultTemplate;
use crate::utils::{Announcement, update_embeds};
use crate::{PostRow, Result};

pub struct JoinInteraction {
    thread: ThreadId,
    user: UserId,
}

impl From<&ComponentInteraction> for JoinInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        Self { thread: value.channel_id.expect_thread(), user: value.user.id }
    }
}

impl From<&CommandInteraction> for JoinInteraction {
    fn from(value: &CommandInteraction) -> Self {
        let Ok((_, sub_options)) = parse_subcommand(value.data.options()) else {
            return Self {
                thread: value.channel_id.expect_thread(),
                user: value.user.id,
            };
        };
        let mut options = parse_options(sub_options);

        let thread = match options.remove("thread") {
            Some(ResolvedValue::Channel(GenericInteractionChannel::Thread(
                thread,
            ))) => thread.id,
            _ => value.channel_id.expect_thread(),
        };

        let user = match options.remove("guardian") {
            Some(ResolvedValue::User(user, _)) => user.id,
            _ => value.user.id,
        };

        Self { thread, user }
    }
}

pub async fn join<'a>(
    http: &'a Http,
    interaction: impl Into<JoinInteraction>,
    pool: &PgPool,
    alternative: bool,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();

    let row = PostRow::join(
        pool,
        interaction.thread.widen(),
        interaction.user,
        alternative,
    )
    .await?;

    let owner = row.owner().to_user(http).await?;

    let embed = update_embeds::<DefaultTemplate>(
        http,
        &row,
        owner.display_name(),
        interaction.thread,
    )
    .await?;

    Announcement::Joined { user: interaction.user, alternative }
        .send(http, interaction.thread)
        .await?;

    Ok((interaction.thread, embed))
}
