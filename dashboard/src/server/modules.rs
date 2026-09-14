use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{app_state, guild_admin_context, server_err},
    crate::server::patreon::fetch_patreon_status,
    crate::server::supersede,
    std::collections::HashMap,
    twilight_model::id::Id,
    zayden_app::modules::{self, Backing, MODULES, ModuleDef, ModuleStates},
};

use crate::dto::ModuleView;

#[cfg(feature = "ssr")]
const fn locked_for(module: &ModuleDef) -> Option<&'static str> {
    match module.backing {
        Backing::Derived => Some(
            "This module is switched on from its own settings page \u{2014} \
             use Configure below.",
        ),
        Backing::Commands | Backing::Settings => None,
    }
}

#[cfg(feature = "ssr")]
fn view(
    module: &ModuleDef,
    states: &ModuleStates,
    settings_flags: &HashMap<&'static str, bool>,
) -> ModuleView {
    let enabled = match module.backing {
        Backing::Commands => states.get(module.id).copied(),
        Backing::Settings | Backing::Derived => {
            settings_flags.get(module.id).copied()
        },
    };

    ModuleView {
        id: module.id.to_string(),
        label: module.label.to_string(),
        description: module.description.to_string(),
        enabled,
        locked: locked_for(module).map(str::to_owned),
    }
}

#[cfg(feature = "ssr")]
async fn settings_flags(
    guild_id: i64,
) -> Result<HashMap<&'static str, bool>, ServerFnError> {
    let app = app_state()?;

    let (ai, patreon) = tokio::try_join!(
        async { app.settings.ai.get(guild_id).await.map_err(server_err) },
        fetch_patreon_status(&app, guild_id),
    )?;

    // Announcements only fire on a live connection with somewhere to post.
    let patreon_on =
        patreon.connected && !patreon.disabled && patreon.channel_id.is_some();

    Ok(HashMap::from([("ai", ai.enabled), ("patreon", patreon_on)]))
}

#[server]
pub async fn list_guild_modules(
    guild: String,
) -> Result<Vec<ModuleView>, ServerFnError> {
    let ctx = guild_admin_context(&guild).await?;

    let (states, flags) = tokio::try_join!(
        async {
            app_state()?.modules.states(ctx.guild_id).await.map_err(server_err)
        },
        settings_flags(ctx.guild_id),
    )?;

    Ok(MODULES.iter().map(|m| view(m, &states, &flags)).collect())
}

#[cfg(feature = "ssr")]
async fn set_settings_enabled(
    module_id: &str,
    guild_id: i64,
    enabled: bool,
) -> Result<(), ServerFnError> {
    match module_id {
        "ai" => app_state()?
            .settings
            .ai
            .update(guild_id, |row| row.enabled = enabled)
            .await
            .map(|_row| ())
            .map_err(server_err),
        _ => Err(ServerFnError::ServerError(format!(
            "module {module_id} has no settings switch"
        ))),
    }
}

#[server]
pub async fn set_module_enabled(
    guild: String,
    module_id: String,
    enabled: bool,
) -> Result<(), ServerFnError> {
    let Some(module) = modules::find(&module_id) else {
        return Err(ServerFnError::ServerError("unknown module".to_string()));
    };

    let ctx = guild_admin_context(&guild).await?;

    let claim = supersede::claim(Id::new(ctx.guild_id.cast_unsigned()), module.id);
    let _turn = claim.wait_for_turn().await;

    if claim.superseded() {
        return Ok(());
    }

    match module.backing {
        Backing::Settings => {
            set_settings_enabled(module.id, ctx.guild_id, enabled).await
        },
        Backing::Commands => app_state()?
            .modules
            .set(ctx.guild_id, module.id, enabled)
            .await
            .map_err(server_err),
        Backing::Derived => Err(ServerFnError::ServerError(format!(
            "{} is switched on from its own settings page, not from this toggle.",
            module.label
        ))),
    }
}
