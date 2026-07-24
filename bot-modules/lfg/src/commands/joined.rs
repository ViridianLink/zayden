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
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use super::Command;
use crate::Result;

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

    pub async fn upcoming(pool: &PgPool, user: UserId) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            JoinedRow,
            r#"
            SELECT
                p.id,
                p.activity,
                p.start_time as "start_time: jiff_sqlx::Timestamp",

                COALESCE(
                    (SELECT array_agg(f.user_id) FROM lfg_fireteam f WHERE f.post_id = p.id),
                    '{}'
                ) AS "fireteam!"

            FROM
                lfg_posts p
            JOIN lfg_fireteam f ON p.id = f.post_id
            WHERE
                f.user_id = $1
            "#,
            as_i64(user.get())
        )
        .fetch_all(pool)
        .await
    }
}

impl Command {
    pub async fn joined(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let posts = JoinedRow::upcoming(pool, interaction.user.id).await?;

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
