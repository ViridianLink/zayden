use std::time::Duration;

use serenity::all::{
    Context,
    CreateCommand,
    ErrorResponse,
    Guild,
    GuildId,
    HttpError,
    StatusCode,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, warn};

use super::Handler;
use crate::{BotState, Result};

const COMMAND_SYNC_ATTEMPTS: u32 = 4;
const COMMAND_SYNC_BACKOFF: Duration = Duration::from_secs(2);

impl Handler {
    pub async fn guild_create(
        &self,
        ctx: &Context,
        guild: &Guild,
        pool: &PgPool,
    ) -> Result<()> {
        let data = ctx.data::<RwLock<BotState>>();

        let (lfg_result, ()) = tokio::join!(
            lfg::events::guild_create::<BotState>(ctx, guild, pool),
            BotState::guild_create(data, guild),
        );
        lfg_result?;

        let commands = self.registry.definitions_for(guild.id);

        set_commands(ctx, guild.id, &commands).await?;
        info!("Registered {}", guild.name);

        Ok(())
    }
}

async fn set_commands(
    ctx: &Context,
    guild_id: GuildId,
    commands: &[CreateCommand<'_>],
) -> serenity::Result<()> {
    let mut backoff = COMMAND_SYNC_BACKOFF;

    for attempt in 1..=COMMAND_SYNC_ATTEMPTS {
        let Err(e) = guild_id.set_commands(&ctx.http, commands).await else {
            return Ok(());
        };

        if attempt == COMMAND_SYNC_ATTEMPTS || !is_transient(&e) {
            return Err(e);
        }

        warn!(
            error = ?e,
            %guild_id,
            attempt,
            ?backoff,
            "command registration failed, retrying",
        );

        sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
    }

    Ok(())
}

fn is_transient(error: &serenity::Error) -> bool {
    let serenity::Error::Http(http) = error else { return false };

    if let HttpError::UnsuccessfulRequest(ErrorResponse { status_code, .. }) = http {
        return status_code.is_server_error()
            || *status_code == StatusCode::TOO_MANY_REQUESTS;
    }

    matches!(http, HttpError::Request(_))
}
