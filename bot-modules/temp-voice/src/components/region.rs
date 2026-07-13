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

use super::{Components, REGION_MENU, REGIONS, resolve_target_channel};
use crate::{Result, TempVoiceError, VoiceChannelManager, VoiceStateCache, actions};

impl Components {
    pub async fn region<Db: Database, Manager: VoiceChannelManager<Db>>(
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

        let options = REGIONS
            .iter()
            .map(|(label, value)| CreateSelectMenuOption::new(*label, *value))
            .collect::<Vec<_>>();

        let menu =
            CreateSelectMenu::new(REGION_MENU, CreateSelectMenuKind::String {
                options: options.into(),
            });

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Select the channel's region")
                        .select_menu(menu)
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn region_menu<Db: Database, Manager: VoiceChannelManager<Db>>(
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
                "region_menu: expected StringSelect interaction".into(),
            ));
        };

        let region = match values.first().map(String::as_str) {
            None | Some("automatic") => None,
            Some(region) => Some(region.to_string()),
        };

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg =
            actions::region(http, channel_id, &row, interaction.user.id, region)
                .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }
}
