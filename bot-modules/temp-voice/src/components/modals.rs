use serenity::all::{
    ComponentInteraction,
    CreateInputText,
    CreateInteractionResponse,
    CreateLabel,
    CreateModal,
    CreateModalComponent,
    EditInteractionResponse,
    Http,
    InputTextStyle,
    ModalInteraction,
};
use sqlx::{Database, Pool};
use zayden_core::parse_modal_components;

use super::{BITRATE, Components, LIMIT, PASSWORD, RENAME, resolve_target_channel};
use crate::{Result, TempVoiceError, VoiceChannelManager, VoiceStateCache, actions};

struct ModalSpec {
    custom_id: &'static str,
    title: &'static str,
    label: &'static str,
    field: &'static str,
    required: bool,
}

async fn open_modal<Db: Database, Manager: VoiceChannelManager<Db>>(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    voice_states: &VoiceStateCache,
    spec: ModalSpec,
) -> Result<()> {
    resolve_target_channel::<Db, Manager>(
        pool,
        voice_states,
        interaction.channel_id,
        interaction.user.id,
    )
    .await?;

    let input = CreateInputText::new(InputTextStyle::Short, spec.field)
        .required(spec.required);

    let modal = CreateModal::new(spec.custom_id, spec.title).components(vec![
        CreateModalComponent::Label(CreateLabel::input_text(spec.label, input)),
    ]);

    interaction
        .create_response(http, CreateInteractionResponse::Modal(modal))
        .await?;

    Ok(())
}

fn modal_field(interaction: &ModalInteraction, field: &str) -> Option<String> {
    let mut inputs = parse_modal_components(interaction.data.components.as_slice());
    inputs.remove(field).and_then(|mut values| values.pop()).map(|v| v.to_string())
}

impl Components {
    pub async fn rename<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_modal::<Db, Manager>(http, interaction, pool, voice_states, ModalSpec {
            custom_id: RENAME,
            title: "Rename Channel",
            label: "New name",
            field: "name",
            required: false,
        })
        .await
    }

    pub async fn rename_submit<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let name = modal_field(interaction, "name")
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("{}'s Channel", interaction.user.name));

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::rename(http, channel_id, &row, interaction.user.id, name)
            .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn limit<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_modal::<Db, Manager>(http, interaction, pool, voice_states, ModalSpec {
            custom_id: LIMIT,
            title: "User Limit",
            label: "User limit (0-99)",
            field: "user_limit",
            required: true,
        })
        .await
    }

    pub async fn limit_submit<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let limit = modal_field(interaction, "user_limit")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .ok_or(TempVoiceError::InvalidNumber)?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::limit(http, channel_id, &row, interaction.user.id, limit)
            .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn bitrate<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_modal::<Db, Manager>(http, interaction, pool, voice_states, ModalSpec {
            custom_id: BITRATE,
            title: "Bitrate",
            label: "Bitrate (kbps)",
            field: "kbps",
            required: true,
        })
        .await
    }

    pub async fn bitrate_submit<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let kbps = modal_field(interaction, "kbps")
            .and_then(|v| v.trim().parse::<i64>().ok())
            .ok_or(TempVoiceError::InvalidNumber)?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg =
            actions::bitrate(http, channel_id, &row, interaction.user.id, kbps)
                .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn password<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_modal::<Db, Manager>(http, interaction, pool, voice_states, ModalSpec {
            custom_id: PASSWORD,
            title: "Channel Password",
            label: "Password",
            field: "password",
            required: true,
        })
        .await
    }

    pub async fn password_submit<Db: Database, Manager: VoiceChannelManager<Db>>(
        http: &Http,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let password = modal_field(interaction, "password")
            .filter(|p| !p.is_empty())
            .ok_or(TempVoiceError::InvalidPassword)?;

        let guild_id = interaction.guild_id.ok_or(TempVoiceError::MissingGuildId)?;

        let (channel_id, row) = resolve_target_channel::<Db, Manager>(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::password::<Db, Manager>(
            http,
            pool,
            guild_id,
            channel_id,
            row,
            interaction.user.id,
            password,
        )
        .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }
}
