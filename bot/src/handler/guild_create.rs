use serenity::all::{Context, Guild};
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use tracing::info;
use zayden_core::ApplicationCommand;

use crate::modules::events::Live;
use crate::modules::lfg::{GuildTable, PostTable};
use crate::modules::{APPLICATION_COMMANDS, llamad2};
use crate::{BRADSTER_GUILD, CtxData, LLAMAD2_GUILD, Result};

use super::Handler;

impl Handler {
    pub async fn guild_create(&self, ctx: &Context, guild: &Guild, pool: &PgPool) -> Result<()> {
        let data = ctx.data::<RwLock<CtxData>>();

        let (lfg_result, _) = tokio::join!(
            lfg::events::guild_create::<CtxData, Postgres, GuildTable, PostTable>(ctx, guild, pool),
            CtxData::guild_create(data, guild),
        );
        lfg_result?;

        let mut commands = APPLICATION_COMMANDS
            .iter()
            .filter(|(name, _)| {
                ![
                    "live",
                    "dungeonreport",
                    "goof",
                    "hello",
                    "playlist",
                    "raidreport",
                    "sensitivity",
                    "socials",
                ]
                .contains(&name.as_str())
            })
            .map(|(_, cmd)| cmd.command())
            .collect::<Vec<_>>();

        match guild.id {
            BRADSTER_GUILD => {
                commands.push(Live {}.command());

                BRADSTER_GUILD.set_commands(&ctx.http, &commands).await?;

                info!("Registered {}", guild.name);
            }
            LLAMAD2_GUILD => {
                commands.extend(llamad2::register());

                LLAMAD2_GUILD.set_commands(&ctx.http, &commands).await?;

                info!("Registered {}", guild.name);
            }
            id => {
                id.set_commands(&ctx.http, &commands).await?;
            }
        }

        Ok(())
    }
}
