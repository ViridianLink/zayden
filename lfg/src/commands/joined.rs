use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serenity::all::{
    ChannelId, CommandInteraction, CreateEmbed, EditInteractionResponse, Http, Mentionable, UserId,
};
use sqlx::{Database, Pool, prelude::FromRow};

use super::Command;

#[async_trait]
pub trait JoinedManager<Db: Database> {
    async fn upcoming(
        pool: &Pool<Db>,
        user: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<JoinedRow>>;
}

#[derive(FromRow)]
pub struct JoinedRow {
    pub id: i64,
    pub activity: String,
    pub start_time: DateTime<Utc>,
    pub fireteam: Vec<i64>,
}

impl JoinedRow {
    pub fn channel_id(&self) -> ChannelId {
        ChannelId::new(self.id as u64)
    }

    pub fn activity(&self) -> &str {
        &self.activity
    }

    pub fn timestamp(&self) -> i64 {
        self.start_time.timestamp()
    }

    pub fn fireteam(&self) -> impl Iterator<Item = UserId> {
        self.fireteam.iter().map(|&id| UserId::new(id as u64))
    }
}

impl Command {
    pub async fn joined<Db: Database, Manager: JoinedManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) {
        interaction.defer_ephemeral(http).await.unwrap();

        let posts = Manager::upcoming(pool, interaction.user.id).await.unwrap();

        let (joined, alternative) = posts
            .into_iter()
            .partition::<Vec<_>, _>(|row| row.fireteam().any(|user| user == interaction.user.id));

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

            embed = embed.field("Joined Posts", values.join("\n\n"), false)
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

            embed = embed.field("Alternative Posts", values.join("\n\n"), false)
        }

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();
    }
}
