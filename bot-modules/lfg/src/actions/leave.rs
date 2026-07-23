use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateEmbed,
    GenericInteractionChannel,
    Http,
    ThreadId,
    UserId,
};
use sqlx::{Database, Pool};
use zayden_core::{optional_option, parse_options, parse_subcommand};

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
        let Ok((_, sub_options)) = parse_subcommand(value.data.options()) else {
            return Self {
                thread: value.channel_id.expect_thread(),
                author: value.user.id,
                user: value.user.id,
            };
        };

        let mut options = parse_options(sub_options);

        let thread = match optional_option::<&GenericInteractionChannel, _>(
            &mut options,
            "thread",
        ) {
            Some(GenericInteractionChannel::Thread(thread)) => thread.id,
            _ => value.channel_id.expect_thread(),
        };

        let user =
            optional_option(&mut options, "guardian").unwrap_or(&value.user).id;

        Self { thread, author: value.user.id, user }
    }
}

impl From<&ComponentInteraction> for LeaveInteraction {
    fn from(value: &ComponentInteraction) -> Self {
        let user = match &value.data.kind {
            ComponentInteractionDataKind::UserSelect { values } => {
                let Some(v) = values.first() else {
                    return Self {
                        thread: value.channel_id.expect_thread(),
                        author: value.user.id,
                        user: value.user.id,
                    };
                };
                *v
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
    user: UserId,
) -> Result<(ThreadId, CreateEmbed<'a>)> {
    let interaction = interaction.into();

    let row = Manager::leave(pool, interaction.thread.widen(), user).await?;

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
