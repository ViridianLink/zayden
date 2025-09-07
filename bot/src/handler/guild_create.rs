use serenity::all::{Context, DiscordJsonError, ErrorResponse, Guild, HttpError, JsonErrorCode};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use zayden_core::ApplicationCommand;

use crate::ctx_data::CtxData;
use crate::modules;
use crate::modules::events::live::Live;
use crate::modules::lfg::{GuildTable, PostTable};
use crate::{BRADSTER_GUILD, Result};

use super::Handler;

impl Handler {
    pub async fn guild_create(ctx: &Context, guild: &Guild, pool: &PgPool) -> Result<()> {
        let data = ctx.data::<RwLock<CtxData>>();

        let commands = modules::global_register(ctx);

        let (_, _, commands_result) = tokio::join!(
            lfg::events::guild_create::<CtxData, Postgres, GuildTable, PostTable>(ctx, guild, pool),
            CtxData::guild_create(data, guild),
            guild.id.set_commands(&ctx.http, &commands),
        );
        match commands_result {
            Ok(_) => {}
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                error:
                    DiscordJsonError {
                        code: JsonErrorCode::InvalidFormBody,
                        errors,
                        ..
                    },
                ..
            }))) if errors
                .first()
                .is_some_and(|e| e.code == "BASE_TYPE_BAD_LENGTH") =>
            {
                let index = errors
                    .first()
                    .unwrap()
                    .path
                    .split_once('.')
                    .unwrap()
                    .0
                    .parse::<usize>()
                    .unwrap();

                let command = commands.get(index).unwrap();

                eprintln!("{errors:?} - {command:?}")
            }
            Err(e) => {
                eprintln!("Unhandled command error: {e:?}")
            }
        }

        if guild.id == 1222360995700150443 {
            println!("Registered Zayden Guild")
        } else if guild.id == BRADSTER_GUILD {
            guild
                .id
                .create_command(&ctx.http, Live::register(ctx).unwrap())
                .await?;

            println!("Registered Bradster Guild");
        }

        Ok(())
    }
}
