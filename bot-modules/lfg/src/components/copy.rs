use serenity::all::{ComponentInteraction, CreateInteractionResponse, CreateModal, Http};
use sqlx::{Database, Pool};

use crate::modals::modal_components;
use crate::{Error, Result};

use super::{Components, EditManager};

impl Components {
    pub async fn copy<Db: Database, Manager: EditManager<Db>>(
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

        let modal = CreateModal::new("lfg_create", "Copy Event").components(row);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await
            .unwrap();

        Ok(())
    }
}
