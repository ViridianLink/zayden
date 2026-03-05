use serenity::all::{
    ComponentInteraction, CreateComponent, CreateInteractionResponse,
    CreateInteractionResponseMessage, Http,
};
use sqlx::{Database, Pool};

use crate::templates::{DefaultTemplate, Template};
use crate::{Error, PostManager, Result};

use super::Components;

impl Components {
    pub async fn settings<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let owner = match Manager::owner(pool, interaction.channel_id).await {
            Ok(owner) => owner,
            Err(sqlx::Error::RowNotFound) => interaction.user.id,
            Err(e) => panic!("{e:?}"),
        };

        if interaction.user.id != owner {
            return Err(Error::PermissionDenied(owner));
        }

        let main_row = DefaultTemplate::main_row();
        let settings_row = DefaultTemplate::settings_row();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().components(vec![
                        CreateComponent::ActionRow(main_row),
                        CreateComponent::ActionRow(settings_row),
                    ]),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }
}
