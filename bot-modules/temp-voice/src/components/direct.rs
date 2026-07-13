use serenity::all::{ComponentInteraction, EditInteractionResponse, Http};
use sqlx::{Database, Pool};

use super::{Components, resolve_target_channel};
use crate::{Result, VoiceChannelManager, VoiceStateCache, actions};

impl Components {
    pub async fn claim<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::claim::<Db, Manager>(
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

    pub async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::delete::<Db, Manager>(
            http,
            pool,
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
}
