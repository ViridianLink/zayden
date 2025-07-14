use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serenity::all::{
    ComponentInteraction,  CreateInteractionResponse, CreateModal, Http, MessageId, UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};

use crate::modals::modal_components;
use crate::{Error, Result};

use super::Components;

#[async_trait]
pub trait EditManager<Db: Database> {
    async fn edit_row(pool: &Pool<Db>, id: impl Into<MessageId> + Send) -> sqlx::Result<EditRow>;
}

#[derive(FromRow)]
pub struct EditRow {
    pub owner: i64,
    pub activity: String,
    pub start_time: DateTime<Utc>,
    pub description: String,
    pub fireteam_size: i16,
    pub timezone: Option<String>,
}

impl EditRow {
    pub fn owner(&self) -> UserId {
        UserId::new(self.owner as u64)
    }

    pub fn start_time(&self) -> DateTime<Tz> {
        let tz = match self.timezone.as_deref() {
            Some(tz) => tz.parse().unwrap_or(Tz::UTC),
            None => Tz::UTC,
        };

        self.start_time.with_timezone(&tz)
    }
}

impl Components {
    pub async fn edit<Db: Database, Manager: EditManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let post = Manager::edit_row(pool, interaction.message.id)
            .await
            .unwrap();

        if interaction.user.id != post.owner() {
            return Err(Error::PermissionDenied(post.owner()));
        }

        let row = modal_components(
            &post.activity,
            post.start_time(),
            post.fireteam_size,
            Some(&post.description),
        );

        let modal = CreateModal::new("lfg_edit", "Edit Event").components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await
            .unwrap();

        Ok(())
    }
}
