use accounts::links::AccountLink;
use accounts::provider::Provider as AccountProvider;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::{page, path_param};
use topcoat::view::{View, component, view};
use tracing::warn;

use super::deadlock::{DeadlockPrefs, deadlock_section, load as load_deadlock};
use crate::auth::{
    AuthError,
    current_user_id,
    db_pool,
    remember_login_next,
    require_user,
};
use crate::components::flash::flash;
use crate::flash::{Flash, FlashKind, set as set_flash, take as take_flash};
use crate::public::layout::public_layout;

path_param!(provider);

pub(super) const PAGE: &str = "/account";
pub(super) const SIGN_IN_PATH: &str = "/auth/discord";

#[page("/account")]
pub(super) async fn account(cx: &Cx) -> Result<impl View> {
    let notice = take_flash(cx);

    let (links, summaries) = match require_user(cx).await {
        Ok(identity) => {
            let links =
                match AccountLink::list_for_user(db_pool(cx)?, identity.user_id)
                    .await
                {
                    Ok(links) => Some(links),
                    Err(error) => {
                        warn!(error = %error, "failed to load linked accounts");
                        None
                    },
                };
            (links, load_deadlock(cx, &identity).await)
        },
        Err(AuthError::Unauthenticated) => {
            remember_login_next(cx, PAGE)?;
            return Err(see_other(SIGN_IN_PATH).into());
        },
        Err(error) => {
            warn!(error = %error, "failed to resolve the signed-in user");
            (None, None)
        },
    };

    Ok(view! {
        public_layout(account_page(
            notice: notice.as_ref(),
            links: links.as_deref(),
            summaries: summaries.as_ref()
        ))
    })
}

#[page(POST "/account/unlink/{provider}")]
pub(super) async fn unlink(cx: &Cx) -> Result<impl View> {
    let provider: &str = path_param::<Provider>(cx);
    unlink_provider(cx, provider).await?;

    Err::<(), _>(see_other(PAGE).into())
}

async fn unlink_provider(cx: &Cx, provider: &str) -> Result<()> {
    let Ok(provider) = provider.parse::<AccountProvider>() else {
        set_flash(cx, FlashKind::Error, "Unknown account provider.")?;
        return Ok(());
    };

    let user_id = match current_user_id(cx).await {
        Ok(user_id) => user_id,
        Err(AuthError::Unauthenticated) => {
            remember_login_next(cx, PAGE)?;
            return Err(see_other(SIGN_IN_PATH).into());
        },
        Err(error) => return Err(error.into()),
    };

    let label = provider.label();
    match AccountLink::delete(db_pool(cx)?, user_id, provider).await {
        Ok(true) => set_flash(
            cx,
            FlashKind::Success,
            &format!("Unlinked your {label} account."),
        )?,
        Ok(false) => set_flash(
            cx,
            FlashKind::Error,
            &format!("No {label} account was linked."),
        )?,
        Err(error) => {
            warn!(error = %error, "failed to unlink an account");
            set_flash(
                cx,
                FlashKind::Error,
                &format!("Couldn't unlink your {label} account. Please try again."),
            )?;
        },
    }

    Ok(())
}

#[component]
async fn account_page(
    notice: Option<&Flash>,
    links: Option<&[AccountLink]>,
    summaries: Option<&DeadlockPrefs>,
) -> Result<impl View> {
    Ok(view! {
        <div class="page account-page">
            <div class="page-header">
                <div>
                    <h1>"Linked accounts"</h1>
                    <p class="page-lead">
                        "Link a game account so Zayden can track it. The game confirms you own the account; Zayden never sees your password."
                    </p>
                </div>
            </div>
            flash(notice: notice)
            match links {
                None => {
                    <p class="error" role="alert">
                        "Your linked accounts couldn't be loaded. Please try again shortly."
                    </p>
                }
                Some(links) => {
                    <ul class="account-list">
                        #[key(provider.as_str())]
                        for provider in AccountProvider::ALL {
                            provider_row(
                                provider: provider,
                                link: links.iter().find(|link| link.provider == provider)
                            )
                        }
                    </ul>
                }
            }
            deadlock_section(prefs: summaries)
        </div>
    })
}

#[component]
async fn provider_row(
    provider: AccountProvider,
    link: Option<&AccountLink>,
) -> Result<impl View> {
    let details = link.map(|link| {
        let (account_id, id64) = link.steam_id().map_or_else(
            || (link.external_id.clone(), link.external_id.clone()),
            |id| (id.account_id().to_string(), id.to_string()),
        );
        (account_id, id64, link.linked_at.strftime("%Y-%m-%d").to_string())
    });
    let unlink_action = format!("/account/unlink/{}", provider.as_str());
    let link_href = format!("/account/link/{}", provider.as_str());
    let link_label = format!("Link {}", provider.label());

    Ok(view! {
        <li class="card account-row">
            <div class="account-main">
                <h2 class="label">(provider.label())</h2>
                match &details {
                    Some((account_id, id64, linked)) => {
                        <p class="account-id">
                            <code>(account_id.as_str())</code>
                            " (SteamID64 "
                            <code>(id64.as_str())</code>
                            ")"
                        </p>
                        <p class="account-meta">
                            "Linked "
                            (linked.as_str())
                        </p>
                    }
                    None => {
                        <p class="account-meta">"Not linked"</p>
                    }
                }
            </div>
            match &details {
                Some(_) => {
                    <form method="post" action=(unlink_action) data-pending="">
                        <button
                            type="submit"
                            class="btn btn-secondary"
                            data-pending-label="Unlinking\u{2026}"
                        >
                            "Unlink"
                        </button>
                    </form>
                }
                None => {
                    <a href=(link_href) rel="external" class="btn btn-primary">
                        (link_label)
                    </a>
                }
            }
        </li>
    })
}
