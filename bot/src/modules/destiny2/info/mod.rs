use async_trait::async_trait;
use endgame_analysis::DestinyPerk;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct Perk;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Perk {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let ResolvedValue::String(perk) = options[0].value else {
            unreachable!("Option must be string")
        };

        let perk = match sqlx::query_as!(
            DestinyPerk,
            "SELECT * FROM destiny_perks WHERE name = $1 LIMIT 1",
            perk
        )
        .fetch_one(pool)
        .await
        {
            Ok(perk) => perk,
            Err(_) => {
                interaction.edit_response(&ctx.http, EditInteractionResponse::new().content("This command is still work in progress. Please make sure the perk is typed __exactly__ how it appears in game (including captalisation).")).await.unwrap();
                return Ok(());
            }
        };

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content(format!("__{}__\n{}", perk.name, perk.description)),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        let cmd = CreateCommand::new("perk")
            .description("Perk information")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "perk", "Perk name")
                    .required(true),
            );

        Ok(cmd)
    }
}
