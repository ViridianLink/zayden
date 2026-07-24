use serenity::all::{
    ComponentInteraction,
    CreateComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Http,
};
use sqlx::PgPool;

use super::Components;
use crate::templates::{DefaultTemplate, Template};
use crate::{LfgError, PostRow, Result};

impl Components {
    pub async fn settings(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let owner = match PostRow::fetch_owner(pool, interaction.channel_id).await {
            Ok(owner) => owner,
            Err(sqlx::Error::RowNotFound) => interaction.user.id,
            Err(e) => return Err(e.into()),
        };

        if interaction.user.id != owner {
            return Err(LfgError::PermissionDenied(owner));
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
            .await?;

        Ok(())
    }
}
