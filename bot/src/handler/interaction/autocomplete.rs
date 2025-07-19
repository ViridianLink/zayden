use serenity::all::{
    CommandInteraction, Context, DiscordJsonError, ErrorResponse, HttpError, JsonErrorCode,
};
use sqlx::PgPool;
use zayden_core::Autocomplete;

use crate::handler::Handler;
use crate::modules::destiny2::endgame_analysis::slash_commands::{TierList, Weapon};
use crate::modules::lfg::Lfg;
use crate::{Error, Result};

impl Handler {
    pub async fn interaction_autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let option = interaction.data.autocomplete().unwrap();

        let result = match interaction.data.name.as_str() {
            "lfg" => Lfg::autocomplete(ctx, interaction, option, pool).await,
            "weapon" => Weapon::autocomplete(ctx, interaction, option, pool).await,
            "tierlist" => TierList::autocomplete(ctx, interaction, option, pool).await,
            _ => {
                println!("Unknown command: {}", interaction.data.name);
                return Ok(());
            }
        };

        match result {
            Ok(_)
            | Err(Error::ZaydenCore(zayden_core::Error::Serenity(serenity::Error::Http(
                HttpError::UnsuccessfulRequest(ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::UnknownWebhook,
                            ..
                        },
                    ..
                }),
            )))) => {}
            Err(e) => {
                eprintln!("Error handling INTERACTION_AUTOCOMPLETE: {e:?}");
            }
        }

        Ok(())
    }
}
