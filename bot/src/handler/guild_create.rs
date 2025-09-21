use serenity::all::{Context, Guild};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use zayden_core::ApplicationCommand;

use crate::modules::events::Live;
use crate::modules::lfg::{GuildTable, PostTable};
use crate::{BRADSTER_GUILD, CtxData, Result};

use super::Handler;

impl Handler {
    pub async fn guild_create(ctx: &Context, guild: &Guild, pool: &PgPool) -> Result<()> {
        let data = ctx.data::<RwLock<CtxData>>();

        let _ = tokio::join!(
            lfg::events::guild_create::<CtxData, Postgres, GuildTable, PostTable>(ctx, guild, pool),
            CtxData::guild_create(data, guild),
        );

        if guild.id == BRADSTER_GUILD {
            BRADSTER_GUILD
                .create_command(&ctx.http, Live::register(ctx).unwrap())
                .await?;
        }

        Ok(())
    }
}
