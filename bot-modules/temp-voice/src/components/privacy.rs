use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    EditInteractionResponse,
    Http,
};
use sqlx::{Database, Pool};

use super::{Components, PRIVACIES, PRIVACY_MENU, resolve_target_channel};
use crate::{Result, TempVoiceError, VoiceChannelManager, VoiceStateCache, actions};

impl Components {
    pub async fn privacy<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let options = PRIVACIES
            .iter()
            .map(|(label, value)| CreateSelectMenuOption::new(*label, *value))
            .collect::<Vec<_>>();

        let menu =
            CreateSelectMenu::new(PRIVACY_MENU, CreateSelectMenuKind::String {
                options: options.into(),
            });

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Select the channel's privacy")
                        .select_menu(menu)
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn privacy_menu<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let ComponentInteractionDataKind::StringSelect { values } =
            &interaction.data.kind
        else {
            return Err(TempVoiceError::Internal(
                "privacy_menu: expected StringSelect interaction".into(),
            ));
        };

        let privacy = values.first().ok_or_else(|| {
            TempVoiceError::Internal("privacy_menu: no privacy selected".into())
        })?;

        let guild_id = interaction.guild_id.ok_or(TempVoiceError::MissingGuildId)?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::privacy(
            http,
            guild_id,
            voice_states,
            channel_id,
            &row,
            interaction.user.id,
            privacy,
        )
        .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }
}
