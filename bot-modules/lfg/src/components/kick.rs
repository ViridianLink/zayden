use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateSelectMenu,
    CreateSelectMenuKind,
    EditInteractionResponse,
    Http,
};
use sqlx::PgPool;

use super::Components;
use crate::{LfgError, PostRow, Result, actions};

impl Components {
    pub async fn kick(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let owner = PostRow::fetch_owner(pool, interaction.channel_id).await?;

        if interaction.user.id != owner {
            return Err(LfgError::PermissionDenied(owner));
        }

        let select_menu =
            CreateSelectMenu::new("lfg_kick_menu", CreateSelectMenuKind::User {
                default_users: None,
            });

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Select the user you want to kick")
                        .select_menu(select_menu)
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }
}

pub struct KickComponent;

impl KickComponent {
    pub async fn run(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let kicked_user = match &interaction.data.kind {
            ComponentInteractionDataKind::UserSelect { values } => {
                let Some(v) = values.first() else {
                    return Err(LfgError::Internal(
                        "KickComponent: UserSelect had no values".into(),
                    ));
                };

                *v
            },
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::StringSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                return Err(LfgError::Internal(
                    "KickComponent: expected UserSelect interaction".into(),
                ));
            },
        };

        let (_, embed) =
            actions::leave(http, interaction, pool, kicked_user).await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
