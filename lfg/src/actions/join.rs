use serenity::all::{
    CommandInteraction, ComponentInteraction, GenericInteractionChannel, Http, Mentionable,
    ResolvedValue, ThreadId, UserId,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::models::Savable;
use crate::templates::DefaultTemplate;
use crate::utils::{Announcement, update_embeds};
use crate::{Join, PostManager, PostRow, Result};

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

pub async fn join<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
    http: &Http,
    interaction: impl Into<JoinInteraction>,
    pool: &Pool<Db>,
    alternative: bool,
) -> Result<String> {
    let interaction = interaction.into();

    let mut row = Manager::row(pool, interaction.thread).await.unwrap();
    row.join(interaction.user, alternative)?;

    let owner = row.owner().to_user(http).await.unwrap();

    update_embeds::<DefaultTemplate>(http, &row, owner.display_name(), interaction.thread).await;
    Announcement::Joined {
        user: interaction.user,
        alternative,
    }
    .send(http, interaction.thread)
    .await;

    Manager::save(pool, row).await.unwrap();

    Ok(format!(
        "You have joined {}",
        interaction.thread.widen().mention()
    ))
}
