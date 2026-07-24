use serenity::all::{
    ComponentInteraction,
    CreateInteractionResponse,
    CreateModal,
    Http,
};
use sqlx::PgPool;

use super::{Components, EditRow};
use crate::modals::modal_components;
use crate::{LfgError, Result};

impl Components {
    pub async fn copy(
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

        let modal = CreateModal::new("lfg_create", "Copy Event").components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }
}
