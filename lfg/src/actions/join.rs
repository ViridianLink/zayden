use serenity::all::{
    CommandInteraction, ComponentInteraction, CreateEmbed, GenericInteractionChannel, Http,
    ResolvedValue, ThreadId, UserId,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::models::Savable;
use crate::templates::DefaultTemplate;
use crate::utils::{Announcement, update_embeds};
use crate::{PostManager, PostRow, Result};

pub struct JoinInteraction {
    thread: ThreadId,
    user: UserId,
}

impl From<&ComponentInteraction> for JoinInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        Self {
            thread: value.channel_id.expect_thread(),
            user: value.user.id,
        }
    }
}

impl From<&CommandInteraction> for JoinInteraction {
    fn from(value: &CommandInteraction) -> Self {
        let ResolvedValue::SubCommand(subcommand) = value.data.options().pop().unwrap().value
        else {
            unreachable!("Option must be subcommand")
        };

        let mut options = parse_options(subcommand);
        let thread = match options.remove("thread") {
            Some(ResolvedValue::Channel(GenericInteractionChannel::Thread(thread))) => thread.id,
            _ => value.channel_id.expect_thread(),
        };
        let user = match options.remove("guardian") {
            Some(ResolvedValue::User(user, _)) => user.id,
            _ => value.user.id,
        };

        Self { thread, user }
    }
}

pub async fn join<'a, Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
    http: &'a Http,
    interaction: impl Into<JoinInteraction>,
    pool: &Pool<Db>,
    alternative: bool,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();

    let row = Manager::join(pool, interaction.thread, interaction.user, alternative).await?;

    let owner = row.owner().to_user(http).await.unwrap();

    let embed =
        update_embeds::<DefaultTemplate>(http, &row, owner.display_name(), interaction.thread)
            .await;

    Announcement::Joined {
        user: interaction.user,
        alternative,
    }
    .send(http, interaction.thread)
    .await;

    Ok((interaction.thread, embed))
}
