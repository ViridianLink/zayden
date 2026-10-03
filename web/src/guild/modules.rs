use std::collections::HashMap;
use std::hash::BuildHasher;

use topcoat::context::Cx;
use twilight_model::id::Id;
use zayden_app::modules::{self, Backing, MODULES, ModuleDef, ModuleStates};

use super::dto::ModuleView;
use super::error::{GuildError, server_err};
use super::form::form_args;
use super::patreon::fetch_patreon_status;
use super::youtube::fetch_youtube_status;
use crate::auth::{AuthError, app_state, guild_admin_context, supersede};

form_args! {
    ModuleToggleForm { guild, module_id, enabled }
}

impl ModuleToggleForm {
    pub fn enabled(&self) -> Result<bool, GuildError> {
        match self.enabled.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(GuildError::InvalidField("enabled")),
        }
    }
}

const fn locked_for(module: &ModuleDef) -> Option<&'static str> {
    match module.backing {
        Backing::Derived => Some(
            "This module is switched on from its own settings page \u{2014} \
             use Configure below.",
        ),
        Backing::Commands | Backing::Settings => None,
    }
}

#[must_use]
pub fn view<S: BuildHasher>(
    module: &ModuleDef,
    states: &ModuleStates,
    settings_flags: &HashMap<&'static str, bool, S>,
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

async fn settings_flags(
    cx: &Cx,
    guild_id: i64,
) -> Result<HashMap<&'static str, bool>, GuildError> {
    let app = app_state(cx)?;

    let (ai, patreon, youtube) = tokio::try_join!(
        async { app.settings.ai.get(guild_id).await.map_err(server_err) },
        fetch_patreon_status(app, guild_id),
        fetch_youtube_status(app, guild_id),
    )?;

    let patreon_on =
        patreon.connected && !patreon.disabled && patreon.channel_id.is_some();
    let youtube_on = youtube.connected && youtube.channel_id.is_some();

    Ok(HashMap::from([
        ("ai", ai.enabled),
        ("patreon", patreon_on),
        ("youtube", youtube_on),
    ]))
}

pub async fn list_guild_modules(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<ModuleView>, GuildError> {
    let ctx = guild_admin_context(cx, guild).await?;

    let (states, flags) = tokio::try_join!(
        async {
            app_state(cx)?.modules.states(ctx.guild_id).await.map_err(server_err)
        },
        settings_flags(cx, ctx.guild_id),
    )?;

    Ok(MODULES.iter().map(|m| view(m, &states, &flags)).collect())
}

async fn set_settings_enabled(
    cx: &Cx,
    module_id: &str,
    guild_id: i64,
    enabled: bool,
) -> Result<(), GuildError> {
    match module_id {
        "ai" => app_state(cx)?
            .settings
            .ai
            .update(guild_id, |row| row.enabled = enabled)
            .await
            .map(|_row| ())
            .map_err(server_err),
        _ => Err(GuildError::NoSettingsSwitch(module_id.to_owned())),
    }
}

pub async fn set_module_enabled(
    cx: &Cx,
    guild: &str,
    module_id: &str,
    enabled: bool,
) -> Result<(), GuildError> {
    let Some(module) = modules::find(module_id) else {
        return Err(GuildError::UnknownModule);
    };

    let ctx = guild_admin_context(cx, guild).await?;
    let guild_ref = Id::new_checked(ctx.guild_id.cast_unsigned())
        .ok_or(AuthError::InvalidGuildId)?;

    let claim = supersede::claim(guild_ref, module.id);
    let _turn = claim.wait_for_turn().await;

    if claim.superseded() {
        return Ok(());
    }

    match module.backing {
        Backing::Settings => {
            set_settings_enabled(cx, module.id, ctx.guild_id, enabled).await
        },
        Backing::Commands => app_state(cx)?
            .modules
            .set(ctx.guild_id, module.id, enabled)
            .await
            .map_err(server_err),
        Backing::Derived => Err(GuildError::DerivedModule(module.label)),
    }
}
