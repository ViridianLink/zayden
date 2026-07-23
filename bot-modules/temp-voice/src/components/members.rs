use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateSelectMenu,
    CreateSelectMenuKind,
    EditInteractionResponse,
    Http,
    UserId,
};
use sqlx::PgPool;

use super::{
    Components,
    KICK_MENU,
    TRANSFER_MENU,
    TRUST_MENU,
    resolve_target_channel,
};
use crate::{Result, TempVoiceError, VoiceStateCache, actions};

async fn open_user_select(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &PgPool,
    voice_states: &VoiceStateCache,
    menu_id: &'static str,
    prompt: &str,
) -> Result<()> {
    resolve_target_channel(
        pool,
        voice_states,
        interaction.channel_id,
        interaction.user.id,
    )
    .await?;

    let menu = CreateSelectMenu::new(menu_id, CreateSelectMenuKind::User {
        default_users: None,
    });

    interaction
        .create_response(
            http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(prompt)
                    .select_menu(menu)
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}

fn selected_user(interaction: &ComponentInteraction) -> Result<UserId> {
    let ComponentInteractionDataKind::UserSelect { values } = &interaction.data.kind
    else {
        return Err(TempVoiceError::Internal(
            "members: expected UserSelect interaction".into(),
        ));
    };

    values
        .first()
        .copied()
        .ok_or_else(|| TempVoiceError::Internal("members: no user selected".into()))
}

impl Components {
    pub async fn transfer(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_user_select(
            http,
            interaction,
            pool,
            voice_states,
            TRANSFER_MENU,
            "Select the user to transfer ownership to",
        )
        .await
    }

    pub async fn transfer_menu(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let target = selected_user(interaction)?;

        let (channel_id, row) = resolve_target_channel(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg = actions::transfer(
            http,
            pool,
            channel_id,
            row,
            interaction.user.id,
            target,
        )
        .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn trust(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_user_select(
            http,
            interaction,
            pool,
            voice_states,
            TRUST_MENU,
            "Select the user to trust",
        )
        .await
    }

    pub async fn trust_menu(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let target = selected_user(interaction)?;

        let (channel_id, row) = resolve_target_channel(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg =
            actions::trust(http, pool, channel_id, row, interaction.user.id, target)
                .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }

    pub async fn kick(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        open_user_select(
            http,
            interaction,
            pool,
            voice_states,
            KICK_MENU,
            "Select the user to kick",
        )
        .await
    }

    pub async fn kick_menu(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
        voice_states: &VoiceStateCache,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let target = selected_user(interaction)?;

        let guild_id = interaction.guild_id.ok_or(TempVoiceError::MissingGuildId)?;

        let (_, row) = resolve_target_channel(
            pool,
            voice_states,
            interaction.channel_id,
            interaction.user.id,
        )
        .await?;

        let msg =
            actions::kick(http, guild_id, &row, interaction.user.id, target).await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().content(msg))
            .await?;

        Ok(())
    }
}
