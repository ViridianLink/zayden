use serenity::all::{ComponentInteraction, CreateInteractionResponseMessage, Http};
use serenity::all::{CreateInteractionResponse, CreateSelectMenu, CreateSelectMenuKind};
use sqlx::Database;
use sqlx::Pool;

use crate::models::post::PostManager;
use crate::{Error, PostRow, Savable};
use crate::{Result, actions};

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
        pool: &Pool<Db>,
    ) -> Result<()> {
        actions::leave::<Db, Manager>(http, interaction, pool)
            .await
            .unwrap();

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await
            .unwrap();

        Ok(())
    }
}
