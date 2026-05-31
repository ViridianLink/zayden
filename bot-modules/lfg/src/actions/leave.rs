use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateEmbed,
    GenericInteractionChannel,
    Http,
    ResolvedValue,
    ThreadId,
    UserId,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::models::Savable;
use crate::templates::DefaultTemplate;
use crate::utils::{Announcement, update_embeds};
use crate::{PostManager, PostRow, Result};

#[expect(
    dead_code,
    reason = "LeaveInteraction fields are not yet consumed after construction"
)]
pub struct LeaveInteraction {
    thread: ThreadId,
    author: UserId,
    user: UserId,
}

impl From<&CommandInteraction> for LeaveInteraction {
    fn from(value: &CommandInteraction) -> Self {
        #[expect(
            clippy::unreachable,
            reason = "lfg command options are always SubCommand"
        )]
        let ResolvedValue::SubCommand(subcommand) = value
            .data
            .options()
            .pop()
            .expect("lfg action always has a subcommand")
            .value
        else {
            unreachable!("lfg command options are always SubCommand")
        };

        let mut options = parse_options(subcommand);
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

        Self { thread, author: value.user.id, user }
    }
}

impl From<&ComponentInteraction> for LeaveInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        let user = match &value.data.kind {
            ComponentInteractionDataKind::UserSelect { values } => {
                *values.first().expect("UserSelect always has at least one value")
            },
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::StringSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => value.user.id,
        };

        Self {
            thread: value.channel_id.expect_thread(),
            author: value.user.id,
            user,
        }
    }
}

pub async fn leave<
    'a,
    Db: Database,
    Manager: PostManager<Db> + Savable<Db, PostRow>,
>(
    http: &'a Http,
    interaction: impl Into<LeaveInteraction>,
    pool: &Pool<Db>,
    user: impl Into<UserId>,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();
    let user = user.into();

    let row = Manager::leave(pool, interaction.thread, user).await?;

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
