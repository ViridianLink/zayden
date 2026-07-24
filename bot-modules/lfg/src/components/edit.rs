use jiff::Zoned;
use jiff::tz::{self, TimeZone};
use jiff_sqlx::Timestamp;
use serenity::all::{
    ComponentInteraction,
    CreateInteractionResponse,
    CreateModal,
    Http,
    MessageId,
    UserId,
};
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

use super::Components;
use crate::modals::modal_components;
use crate::{LfgError, Result};

#[derive(FromRow)]
pub struct EditRow {
    pub owner_id: i64,
    pub activity: String,
    pub start_time: Timestamp,
    pub description: String,
    pub fireteam_size: i16,
    pub timezone: Option<String>,
}

impl EditRow {
    #[must_use]
    pub const fn owner(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    #[must_use]
    pub fn start_time(&self) -> Zoned {
        let tz = self
            .timezone
            .as_deref()
            .and_then(|s| tz::db().get(s).ok())
            .unwrap_or(TimeZone::UTC);

        self.start_time.to_jiff().to_zoned(tz)
    }

    pub async fn get(pool: &PgPool, id: MessageId) -> sqlx::Result<Self> {
        let id = as_i64(id.get());

        sqlx::query_as!(
            EditRow,
            r#"
            SELECT
                p.owner_id AS "owner_id!",
                p.activity AS "activity!",
                p.start_time AS "start_time!: jiff_sqlx::Timestamp",
                p.description AS "description!",
                p.fireteam_size AS "fireteam_size!",
                u.timezone AS "timezone?"
            FROM
                lfg_posts AS p
            LEFT JOIN
                lfg_user_settings AS u ON p.owner_id = u.id
            WHERE
                p.id = $1
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }
}

impl Components {
    pub async fn edit(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let post = EditRow::get(pool, interaction.message.id).await?;

        if interaction.user.id != post.owner() {
            return Err(LfgError::PermissionDenied(post.owner()));
        }

        let row = modal_components(
            &post.activity,
            &post.start_time(),
            post.fireteam_size,
            Some(&post.description),
        );

        let modal = CreateModal::new("lfg_edit", "Edit Event").components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }
}
