use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serenity::all::{
    CommandId,
    CommandPermissionType,
    CreateCommand,
    Guild,
    GuildId,
    Http,
    RoleId,
};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::sleep;
use tracing::{error, warn};
use zayden_app::events::AppEvent;
use zayden_app::modules::ModuleStates;
use zayden_app::state::AppState;
use zayden_core::is_transient;

use crate::{CommandRegistry, Result};

const COMMAND_SYNC_ATTEMPTS: u32 = 4;
const COMMAND_SYNC_BACKOFF: Duration = Duration::from_secs(2);

pub async fn seed(
    http: &Http,
    app: &AppState,
    registry: &CommandRegistry,
    guild: &Guild,
) -> Result<Arc<ModuleStates>> {
    let guild_id = guild.id.get().cast_signed();

    let imported = if app.modules.needs_import(guild_id).await? {
        imported_disabled(http, registry, guild.id).await?
    } else {
        HashSet::new()
    };

    let joined_at =
        jiff::Timestamp::from_millisecond(guild.joined_at.unix_timestamp_millis())?;

    Ok(app.modules.seed(guild_id, joined_at, &imported).await?)
}

async fn imported_disabled(
    http: &Http,
    registry: &CommandRegistry,
    guild_id: GuildId,
) -> serenity::Result<HashSet<&'static str>> {
    let (commands, permissions) = tokio::try_join!(
        guild_id.get_commands(http),
        guild_id.get_commands_permissions(http),
    )?;

    let everyone = RoleId::new(guild_id.get());

    let denied = permissions
        .iter()
        .filter(|command| {
            command.permissions.iter().any(|p| {
                !p.permission
                    && p.kind == CommandPermissionType::Role
                    && p.id.to_role_id() == everyone
            })
        })
        .map(|command| command.id)
        .collect::<HashSet<CommandId>>();

    let ids = commands
        .iter()
        .map(|command| (command.name.as_str(), command.id))
        .collect::<HashMap<_, _>>();

    Ok(registry
        .commands_by_module()
        .into_iter()
        .filter(|(_module, names)| {
            let known = names
                .iter()
                .filter_map(|name| ids.get(name.as_ref()))
                .collect::<Vec<_>>();

            !known.is_empty() && known.iter().all(|id| denied.contains(*id))
        })
        .map(|(module, _names)| module)
        .collect())
}

pub async fn sync_commands(
    http: &Http,
    registry: &CommandRegistry,
    guild_id: GuildId,
    states: &ModuleStates,
) -> serenity::Result<()> {
    let commands = registry.definitions_for(guild_id, states);
    set_commands(http, guild_id, &commands).await
}

async fn set_commands(
    http: &Http,
    guild_id: GuildId,
    commands: &[CreateCommand<'_>],
) -> serenity::Result<()> {
    let mut backoff = COMMAND_SYNC_BACKOFF;

    for attempt in 1..=COMMAND_SYNC_ATTEMPTS {
        let Err(e) = guild_id.set_commands(http, commands).await else {
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

async fn resync(
    http: &Http,
    app: &AppState,
    registry: &CommandRegistry,
    guild_id: GuildId,
) -> Result<()> {
    let states = app.modules.refresh(guild_id.get().cast_signed()).await?;
    sync_commands(http, registry, guild_id, &states).await?;
    Ok(())
}

pub fn spawn_listener(
    http: Arc<Http>,
    app: Arc<AppState>,
    registry: Arc<CommandRegistry>,
) {
    tokio::spawn(async move {
        let mut rx = app.subscribe();

        loop {
            match rx.recv().await {
                Ok(AppEvent::ModulesChanged(guild_id)) => {
                    if guild_id == 0 {
                        continue;
                    }

                    let guild_id = GuildId::new(guild_id);

                    if let Err(e) = resync(&http, &app, &registry, guild_id).await {
                        error!(
                            error = ?e,
                            %guild_id,
                            "failed to re-register commands after a module change"
                        );
                    }
                },
                Ok(
                    AppEvent::ConfigChanged(_)
                    | AppEvent::EntitlementChanged(_)
                    | AppEvent::PatreonPost(_)
                    | AppEvent::YoutubeUpload(_)
                    | AppEvent::HostingPaid(_),
                ) => {},
                Err(RecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "module listener lagged; missed module changes apply on \
                         the next restart"
                    );
                },
                Err(RecvError::Closed) => break,
            }
        }
    });
}
