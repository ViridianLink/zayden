use serenity::all::{ComponentInteraction, EditInteractionResponse, Http};
use sqlx::PgPool;

use super::{Components, resolve_target_channel};
use crate::{Result, VoiceStateCache, actions};

impl Components {
    pub async fn claim(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (channel_id, row) = resolve_target_channel(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::claim(
            http,
            pool,
            voice_states,
            channel_id,
            row,
            interaction.user.id,
        )
        .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn delete(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (channel_id, row) = resolve_target_channel(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::delete(http, pool, channel_id, row, interaction.user.id)
            .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }
}
