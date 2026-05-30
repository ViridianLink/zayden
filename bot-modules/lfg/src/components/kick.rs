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
use sqlx::{Database, Pool};

use super::Components;
use crate::models::post::PostManager;
use crate::{Error, PostRow, Result, Savable, actions};

impl Components {
    pub async fn kick<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let owner = Manager::owner(pool, interaction.channel_id).await?;

        if interaction.user.id != owner {
            return Err(Error::PermissionDenied(owner));
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
    #[expect(
        clippy::unreachable,
        reason = "kick is only registered for UserSelect interactions"
    )]
    pub async fn run<
        Db: Database,
        Manager: PostManager<Db> + Savable<Db, PostRow>,
    >(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let kicked_user = match &interaction.data.kind {
            ComponentInteractionDataKind::UserSelect { values } => {
                *values.first().expect("UserSelect always has at least one value")
            },
            ComponentInteractionDataKind::Button
            | ComponentInteractionDataKind::StringSelect { .. }
            | ComponentInteractionDataKind::RoleSelect { .. }
            | ComponentInteractionDataKind::MentionableSelect { .. }
            | ComponentInteractionDataKind::ChannelSelect { .. }
            | ComponentInteractionDataKind::Unknown(_) => {
                unreachable!("KickComponent expects a UserSelect interaction")
            },
        };

        let (_, embed) =
            actions::leave::<Db, Manager>(http, interaction, pool, kicked_user)
                .await?;

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
