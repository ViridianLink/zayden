use serenity::all::{
    CommandInteraction, ComponentInteraction, ComponentInteractionDataKind, CreateEmbed,
    GenericInteractionChannel, Http, ResolvedValue, ThreadId, UserId,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{
    PostManager, PostRow, Result,
    models::Savable,
    templates::DefaultTemplate,
    utils::{Announcement, update_embeds},
};

#[allow(dead_code)]
pub struct LeaveInteraction {
    thread: ThreadId,
    author: UserId,
    user: UserId,
}

impl From<&CommandInteraction> for LeaveInteraction {
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

        Self {
            thread,
            author: value.user.id,
            user,
        }
    }
}

impl From<&ComponentInteraction> for LeaveInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        let user = match &value.data.kind {
            ComponentInteractionDataKind::UserSelect { values } => *values.first().unwrap(),
            _ => value.user.id,
        };

        Self {
            thread: value.channel_id.expect_thread(),
            author: value.user.id,
            user,
        }
    }
}

pub async fn leave<'a, Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
    http: &'a Http,
    interaction: impl Into<LeaveInteraction>,
    pool: &Pool<Db>,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();

    let row = Manager::leave(pool, interaction.thread, interaction.user)
        .await
        .unwrap();

    let owner = row.owner().to_user(http).await.unwrap();

    let embed =
        update_embeds::<DefaultTemplate>(http, &row, owner.display_name(), interaction.thread)
            .await;

    Announcement::Left(interaction.user)
        .send(http, interaction.thread)
        .await;

    Ok((interaction.thread, embed))
}
