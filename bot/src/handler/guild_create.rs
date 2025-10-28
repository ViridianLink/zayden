use serenity::all::{Context, Guild};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use tracing::info;
use zayden_core::ApplicationCommand;

use crate::modules::events::Live;
use crate::modules::lfg::{GuildTable, PostTable};
use crate::{BRADSTER_GUILD, CtxData, LLAMAD2_GUILD, Result, ZAYDEN_GUILD, modules};

use super::Handler;

impl Handler {
    pub async fn guild_create(ctx: &Context, guild: &Guild, pool: &PgPool) -> Result<()> {
        if guild.id == BRADSTER_GUILD || guild.id == ZAYDEN_GUILD {
            info!("Registered {}", guild.name);
        }

        let data = ctx.data::<RwLock<CtxData>>();

        let _ = tokio::join!(
            lfg::events::guild_create::<CtxData, Postgres, GuildTable, PostTable>(ctx, guild, pool),
            CtxData::guild_create(data, guild),
        );

        match guild.id {
            BRADSTER_GUILD => {
                BRADSTER_GUILD
                    .create_command(&ctx.http, Live::register(ctx).unwrap())
                    .await?;
            }
            LLAMAD2_GUILD => {
                let iter = modules::llamad2::register(ctx)
                    .map(|c| LLAMAD2_GUILD.create_command(&ctx.http, c));

                futures::future::join_all(iter).await;
            }
            _ => {}
        }

        Ok(())
    }
}
