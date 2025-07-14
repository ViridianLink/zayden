use serenity::all::{Context, Guild};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use zayden_core::SlashCommand;

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

        let (_, _, commands) = tokio::join!(
            lfg::events::guild_create::<CtxData, Postgres, GuildTable, PostTable>(ctx, guild, pool),
            CtxData::guild_create(data, guild),
            guild.id.set_commands(&ctx.http, &commands),
        );
        commands?;

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
