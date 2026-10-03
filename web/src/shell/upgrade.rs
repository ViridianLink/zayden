use std::cmp::Ordering;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{StatusCode, page};
use topcoat::view::{View, component, suspense, view};

use super::chrome::app_shell;
use crate::components::icons::{Icon, icon};
use crate::components::skeleton::{SkeletonShape, skeleton};
use crate::guild::GuildError;
use crate::guild::dto::Tier;
use crate::guild::kofi::{KofiEmailForm, link_kofi_email};
use crate::guild::tier::get_user_tier;
use crate::util::server_error_text;

#[derive(Debug, Clone, PartialEq, Eq)]
struct KofiSubmission {
    email: Option<String>,
    result: std::result::Result<(), String>,
}

#[page("/upgrade")]
pub(super) async fn upgrade() -> Result<impl View> {
    Ok(view! { app_shell(upgrade_page()) })
}

/// Links a Ko-fi email and re-renders the page at the same URL with the
/// result under the form.
#[page(POST "/upgrade")]
pub(super) async fn link_kofi(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let email = pairs
        .iter()
        .find(|(name, _)| name == "email")
        .map(|(_, value)| value.clone());
    let result = link(cx, pairs).await.map_err(server_error_text);
    let status = if result.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    let submission = KofiSubmission { email, result };

    Ok(view! {
        (status)
        app_shell(upgrade_page(submission: Some(submission)))
    })
}

async fn link(
    cx: &Cx,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    link_kofi_email(cx, &KofiEmailForm::from_pairs(pairs)?).await
}

#[component]
async fn upgrade_page(
    #[default] submission: Option<KofiSubmission>,
) -> Result<impl View> {
    let (email, result) = match submission {
        Some(KofiSubmission { email, result }) => (email, Some(result)),
        None => (None, None),
    };

    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Upgrade your plan"</h1>
                    <p class="page-lead">
                        "Paid tiers are cost-recovery: they unlock the features that cost real money to run. Everything else stays free."
                    </p>
                </div>
            </div>

            <ul class="pro-features">
                <li class="pro-feature">
                    icon(name: Icon::Music)
                    <strong>"Music 24/7"</strong>
                    <span>
                        "Keep the bot connected with stay-connected and autoplay."
                    </span>
                </li>
                <li class="pro-feature">
                    icon(name: Icon::Sparkles)
                    <strong>"AI replies"</strong>
                    <span>
                        "High-quality model responses instead of the free tier."
                    </span>
                </li>
                <li class="pro-feature">
                    icon(name: Icon::Gauge)
                    <strong>"Faster & longer Palworld uploads"</strong>
                    <span>"Shorter upload cooldown and larger save quota."</span>
                </li>
                <li class="pro-feature">
                    icon(name: Icon::Message)
                    <strong>"Lower greeting cooldowns"</strong>
                    <span>
                        "Drop the per-member and server-wide limits on "
                        <code>"/good"</code>
                        " - for servers busy enough to hit them."
                    </span>
                </li>
            </ul>

            suspense(
                fallback: view! {
                    <div class="skeleton-grid">
                        skeleton(shape: SkeletonShape::Panel, count: 2)
                    </div>
                },
                plan_ladder()
            )

            <div class="card">
                <p class="label">"Link your Ko-fi email"</p>
                <p class="page-lead">
                    "Connect the email you subscribe with on Ko-fi so your paid membership follows your Discord account."
                </p>
                <form method="post" action="/upgrade" data-pending="">
                    <div class="kofi-link-form">
                        <input
                            class="input"
                            type="email"
                            name="email"
                            placeholder="you@example.com"
                            required=""
                            value=(email)
                        >
                        <button type="submit" class="btn btn-primary">
                            "Link email"
                        </button>
                    </div>
                </form>
                match result {
                    Some(Ok(())) => <p class="success">"Ko-fi email linked."</p>,
                    Some(Err(error)) => <p class="error">(error)</p>,
                    None => {

                    }
                }
            </div>
        </div>
    })
}

/// The paid plans against the viewer's tier (free when signed out). Nothing
/// renders when the tier lookup fails.
#[component]
async fn plan_ladder(cx: &Cx) -> Result<impl View> {
    let info = get_user_tier(cx).await.ok();

    Ok(view! {
        if let Some(info) = info {
            let current = info.tier.unwrap_or(Tier::Free);
            <div class="plan-ladder">
                #[key(plan.css_suffix())]
                for plan in Tier::PAID_LADDER {
                    <div class=(format!("plan-card plan-{}", plan.css_suffix()))>
                        <div class="plan-head">
                            <span
                                class=(format!("tier-badge tier-{}", plan.css_suffix()))
                            >
                                (plan.label())
                            </span>
                            <span class="plan-price">
                                (plan.price())
                                <small>"/mo"</small>
                            </span>
                        </div>
                        <ul class="plan-specs">
                            <li>
                                <strong>
                                    (plan.upload_limit_mb())
                                    " MB"
                                </strong>
                                " Palworld save uploads"
                            </li>
                            <li>
                                <strong>(plan.upload_cooldown())</strong>
                                " upload cooldown"
                            </li>
                        </ul>
                        match current.cmp(&plan) {
                            Ordering::Equal => <span class="plan-current">
                                "Your plan"
                            </span>,
                            Ordering::Greater => <span class="plan-included">
                                "Included"
                            </span>,
                            Ordering::Less => {
                                if let Some(url) = &info.upgrade_url {
                                    <a
                                        href=(url.as_str())
                                        class="btn btn-primary"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        (format!("Get {}", plan.label()))
                                    </a>
                                }
                            }
                        }
                    </div>
                }
            </div>
            <div class="upgrade-actions">
                <a href="/invite" rel="external" class="btn btn-secondary">
                    "Subscribe via Discord"
                </a>
            </div>
        }
    })
}
