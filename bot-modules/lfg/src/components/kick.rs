use serenity::all::EditInteractionResponse;
use serenity::all::{
    ComponentInteraction, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateSelectMenu, CreateSelectMenuKind, Http,
};
use sqlx::Database;
use sqlx::Pool;

use crate::models::post::PostManager;
use crate::{Error, PostRow, Result, Savable, actions};

use super::Components;

impl Components {
    pub async fn kick<Db: Database, Manager: PostManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let owner = Manager::owner(pool, interaction.channel_id).await.unwrap();

        if interaction.user.id != owner {
            return Err(Error::PermissionDenied(owner));
        }

        let select_menu = CreateSelectMenu::new(
            "lfg_kick_menu",
            CreateSelectMenuKind::User {
                default_users: None,
            },
        );

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
            .await
            .unwrap();

        Ok(())
    }
}

pub struct KickComponent;

impl KickComponent {
    pub async fn run<Db: Database, Manager: PostManager<Db> + Savable<Db, PostRow>>(
        http: &Http,
        interaction: &ComponentInteraction,
        _pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await?;

        todo!("Parse kick component");

        #[allow(unreachable_code)]
        let (_, embed) = actions::leave::<Db, Manager>(http, interaction, _pool, &interaction.user)
            .await
            .unwrap();

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
