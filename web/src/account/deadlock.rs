use deadlock::db::{guild_members, modules, user_settings};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::page;
use topcoat::view::{View, ViewExt, component, view};
use tracing::warn;

use super::page::{PAGE, SIGN_IN_PATH};
use crate::auth::{
    AuthError,
    app_state,
    db_pool,
    remember_login_next,
    require_user,
    user_guilds,
};
use crate::flash::{FlashKind, set as set_flash};
use crate::state::SessionIdentity;

const DM_ACTION: &str = "/account/deadlock";
const SERVERS_ACTION: &str = "/account/deadlock/servers";
const DM_FIELD: &str = "dm_summaries";
const SHARE_PREFIX: &str = "share_";

pub(super) struct ServerChoice {
    id: i64,
    name: String,
    shared: bool,
}

pub(super) struct DeadlockPrefs {
    dm_summaries: bool,
    servers: Vec<ServerChoice>,
}

#[must_use]
pub fn shared_selection(
    allowed: &[i64],
    form: &[(String, String)],
) -> Vec<(i64, bool)> {
    allowed
        .iter()
        .map(|&guild_id| {
            let field = format!("{SHARE_PREFIX}{guild_id}");
            (guild_id, form.iter().any(|(name, _)| *name == field))
        })
        .collect()
}

pub(super) async fn load(
    cx: &Cx,
    identity: &SessionIdentity,
) -> Option<DeadlockPrefs> {
    match load_prefs(cx, identity).await {
        Ok(prefs) => Some(prefs),
        Err(error) => {
            warn!(error = %error, "failed to load Deadlock summary preferences");
            None
        },
    }
}

async fn load_prefs(
    cx: &Cx,
    identity: &SessionIdentity,
) -> std::result::Result<DeadlockPrefs, AuthError> {
    let pool = db_pool(cx)?;
    let allowed = allowed_guilds(cx, identity).await?;
    let (dm_summaries, shared) = tokio::try_join!(
        user_settings::dm_summaries(pool, identity.user_id),
        guild_members::shared_guilds(pool, identity.user_id),
    )?;

    Ok(DeadlockPrefs {
        dm_summaries,
        servers: allowed
            .into_iter()
            .map(|(id, name)| ServerChoice {
                id,
                name,
                shared: shared.contains(&id),
            })
            .collect(),
    })
}

async fn allowed_guilds(
    cx: &Cx,
    identity: &SessionIdentity,
) -> std::result::Result<Vec<(i64, String)>, AuthError> {
    let guilds = user_guilds(cx, identity).await?;
    let ids: Vec<i64> =
        guilds.iter().map(|guild| guild.id.get().cast_signed()).collect();
    let enabled = modules::enabled_guilds(
        db_pool(cx)?,
        &ids,
        app_state(cx)?.zayden_id.cast_signed(),
    )
    .await?;

    let mut allowed: Vec<(i64, String)> = guilds
        .iter()
        .map(|guild| (guild.id.get().cast_signed(), guild.name.clone()))
        .filter(|(id, _)| enabled.contains(id))
        .collect();
    allowed.sort_by_cached_key(|(_, name)| name.to_lowercase());

    Ok(allowed)
}

async fn signed_in(cx: &Cx) -> Result<SessionIdentity> {
    match require_user(cx).await {
        Ok(identity) => Ok(identity),
        Err(AuthError::Unauthenticated) => {
            remember_login_next(cx, PAGE)?;
            Err(see_other(SIGN_IN_PATH).into())
        },
        Err(error) => Err(error.into()),
    }
}

#[page(POST "/account/deadlock")]
pub(super) async fn save_dm(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let identity = signed_in(cx).await?;
    let enabled = pairs.iter().any(|(name, _)| name == DM_FIELD);

    match user_settings::set_dm_summaries(db_pool(cx)?, identity.user_id, enabled)
        .await
    {
        Ok(()) => {
            set_flash(cx, FlashKind::Success, "Deadlock summary settings saved.")?;
        },
        Err(error) => {
            warn!(error = %error, "failed to save the Deadlock DM preference");
            set_flash(
                cx,
                FlashKind::Error,
                "Couldn't save your Deadlock summary settings. Please try again.",
            )?;
        },
    }

    Err::<(), _>(see_other(PAGE).into())
}

#[page(POST "/account/deadlock/servers")]
pub(super) async fn save_servers(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let identity = signed_in(cx).await?;

    match share_selected(cx, &identity, &pairs).await {
        Ok(()) => set_flash(
            cx,
            FlashKind::Success,
            "Server sharing for Deadlock summaries saved.",
        )?,
        Err(error) => {
            warn!(error = %error, "failed to save Deadlock server sharing");
            set_flash(
                cx,
                FlashKind::Error,
                "Couldn't save your server sharing choices. Please try again.",
            )?;
        },
    }

    Err::<(), _>(see_other(PAGE).into())
}

async fn share_selected(
    cx: &Cx,
    identity: &SessionIdentity,
    pairs: &[(String, String)],
) -> std::result::Result<(), AuthError> {
    let pool = db_pool(cx)?;
    let allowed: Vec<i64> =
        allowed_guilds(cx, identity).await?.into_iter().map(|(id, _)| id).collect();

    for (guild_id, shared) in shared_selection(&allowed, pairs) {
        guild_members::set_share_summaries(pool, guild_id, identity.user_id, shared)
            .await?;
    }

    Ok(())
}

#[component]
pub(super) async fn deadlock_section(
    prefs: Option<&DeadlockPrefs>,
) -> Result<impl View> {
    Ok(view! {
        <section class="account-deadlock" aria-labelledby="account-deadlock-title">
            <h2 class="label" id="account-deadlock-title">"Deadlock"</h2>
            <p class="page-lead">
                "Summaries need a linked Steam account and appear once deadlock-api has recorded the match. Friending a deadlock-api bot keeps your match history current."
            </p>
            match prefs {
                None => {
                    <p class="error" role="alert">
                        "Your Deadlock settings couldn't be loaded. Please try again shortly."
                    </p>
                }
                Some(prefs) => {
                    <form
                        class="card account-form"
                        method="post"
                        action=(DM_ACTION)
                        data-pending=""
                    >
                        <label class="account-check">
                            <input
                                type="checkbox"
                                name=(DM_FIELD)
                                checked=(prefs.dm_summaries)
                            >
                            <span>"DM me a summary after each match"</span>
                        </label>
                        <button
                            type="submit"
                            class="btn btn-secondary"
                            data-pending-label="Saving\u{2026}"
                        >
                            "Save"
                        </button>
                    </form>
                    <h3 class="label">"Share summaries in servers"</h3>
                    if prefs.servers.is_empty() {
                        <p class="account-meta">
                            "None of your servers use Zayden's Deadlock module."
                        </p>
                    } else {
                        <form
                            class="card account-form"
                            method="post"
                            action=(SERVERS_ACTION)
                            data-pending=""
                        >
                            <ul class="account-checks">
                                #[key(server.id)]
                                for server in &prefs.servers {
                                    <li>
                                        <label class="account-check">
                                            <input
                                                type="checkbox"
                                                name=(format!("{SHARE_PREFIX}{}", server.id))
                                                checked=(server.shared)
                                            >
                                            <span>(server.name.as_str())</span>
                                        </label>
                                    </li>
                                }
                            </ul>
                            <button
                                type="submit"
                                class="btn btn-secondary"
                                data-pending-label="Saving\u{2026}"
                            >
                                "Save servers"
                            </button>
                        </form>
                    }
                }
            }
        </section>
    }
    .boxed())
}
