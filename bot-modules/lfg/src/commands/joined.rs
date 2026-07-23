use async_trait::async_trait;
use jiff_sqlx::Timestamp;
use serenity::all::{
    ChannelId,
    CommandInteraction,
    CreateEmbed,
    EditInteractionResponse,
    Http,
    Mentionable,
    UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use zayden_core::as_u64;

use super::Command;
use crate::Result;

#[async_trait]
pub trait JoinedManager<Db: Database> {
    async fn upcoming(pool: &Pool<Db>, user: UserId)
    -> sqlx::Result<Vec<JoinedRow>>;
}

#[derive(FromRow)]
pub struct JoinedRow {
    pub id: i64,
    pub activity: String,
    pub start_time: Timestamp,
    pub fireteam: Vec<i64>,
}

impl JoinedRow {
    #[must_use]
    pub const fn channel_id(&self) -> ChannelId {
        ChannelId::new(as_u64(self.id))
    }

    #[must_use]
    pub fn activity(&self) -> &str {
        &self.activity
    }

    #[must_use]
    pub fn timestamp(&self) -> jiff::Timestamp {
        self.start_time.to_jiff()
    }

    pub fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(as_u64(id)))
    }
}

impl Command {
    pub async fn joined<Db: Database, Manager: JoinedManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let posts = Manager::upcoming(pool, interaction.user.id).await?;

        let (joined, alternative) =
            posts.into_iter().partition::<Vec<_>, _>(|row| {
                row.fireteam().any(|user| user == interaction.user.id)
            });

        let mut embed = CreateEmbed::new().title("Joined LFG Events");

        if !joined.is_empty() {
            let values = joined
                .into_iter()
                .map(|row| {
                    format!(
                        "{0}\n<t:{1}> (<t:{1}:R>)\n{2}",
                        row.activity(),
                        row.timestamp(),
                        row.channel_id().mention()
                    )
                })
                .collect::<Vec<_>>();

            embed = embed.field("Joined Posts", values.join("\n\n"), false);
        }

        if !alternative.is_empty() {
            let values = alternative
                .into_iter()
                .map(|row| {
                    format!(
                        "{0}\n<t:{1}> (<t:{1}:R>)\n{2}",
                        row.activity(),
                        row.timestamp(),
                        row.channel_id().mention()
                    )
                })
                .collect::<Vec<_>>();

            embed = embed.field("Alternative Posts", values.join("\n\n"), false);
        }

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
