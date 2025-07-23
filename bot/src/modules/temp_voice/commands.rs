use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use temp_voice::VoiceCommand;
use zayden_core::SlashCommand;

use crate::sqlx_lib::GuildTable;
use crate::{CtxData, Error, Result};

use super::VoiceChannelTable;

pub struct Voice;

#[async_trait]
impl SlashCommand<Error, Postgres> for Voice {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        VoiceCommand::run::<CtxData, Postgres, GuildTable, VoiceChannelTable>(
            ctx,
            interaction,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(VoiceCommand::register())
    }
}
