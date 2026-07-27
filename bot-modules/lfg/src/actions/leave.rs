use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    CreateEmbed,
    GenericInteractionChannel,
    Http,
    ThreadId,
    UserId,
};
use sqlx::PgPool;
use zayden_core::{optional_option, parse_options, parse_subcommand};

use crate::templates::DefaultTemplate;
use crate::utils::{Announcement, update_embeds};
use crate::{PostRow, Result};

pub struct LeaveInteraction {
    thread: ThreadId,
}

impl From<&CommandInteraction> for LeaveInteraction {
    fn from(value: &CommandInteraction) -> Self {
        let Ok((_, sub_options)) = parse_subcommand(value.data.options()) else {
            return Self { thread: value.channel_id.expect_thread() };
        };

        let mut options = parse_options(sub_options);

        let thread = match optional_option::<&GenericInteractionChannel, _>(
            &mut options,
            "thread",
        ) {
            Some(GenericInteractionChannel::Thread(thread)) => thread.id,
            _ => value.channel_id.expect_thread(),
        };

        Self { thread }
    }
}

impl From<&ComponentInteraction> for LeaveInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        Self { thread: value.channel_id.expect_thread() }
    }
}

pub async fn leave<'a>(
    http: &'a Http,
    interaction: impl Into<LeaveInteraction>,
    pool: &PgPool,
    user: UserId,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();

    let row = PostRow::leave(pool, interaction.thread.widen(), user).await?;

    let owner = row.owner().to_user(http).await?;

    let embed = update_embeds::<DefaultTemplate>(
        http,
        &row,
        owner.display_name(),
        interaction.thread,
    )
    .await?;

    Announcement::Left(user).send(http, interaction.thread).await?;

    Ok((interaction.thread, embed))
}
