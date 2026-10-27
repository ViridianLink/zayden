use std::cmp::Ordering;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::{StatusCode, page};
use topcoat::view::{View, ViewExt, attributes, class, component, suspense, view};
use url::Url;

use crate::app::guild::guild_id::overview::plain;
use crate::app::public::layout::public_layout;
use crate::components::alert::{AlertVariant, alert, alert_title};
use crate::components::badge::{BadgeVariant, badge_variants};
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::field_row::{describedby, field_row};
use crate::components::flash::flash;
use crate::components::icons::{Icon, icon};
use crate::components::input::input;
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::flash::{Flash, FlashKind, set_in as set_flash, take as take_flash};
use crate::guild::GuildError;
use crate::guild::dto::Tier;
use crate::guild::kofi::{KofiEmailForm, link_kofi_email};
use crate::guild::tier::get_user_tier;

const PAGE: &str = "/upgrade";
const KOFI_SECTION: &str = "kofi";
const EMAIL_ID: &str = "kofi-email";
const LINKED: &str = "Ko-fi email linked. Your paid membership now follows your \
                      Discord account.";
const EMAIL_HELP: &str = "The email you pay with on Ko-fi. It is linked to the \
                          Discord account you are signed in with.";

#[derive(Debug, Clone, PartialEq, Eq)]
struct KofiFailure {
    email: String,
    message: String,
    on_field: bool,
}

impl KofiFailure {
    fn new(email: String, error: &GuildError) -> Self {
        let (message, on_field) = if *error == GuildError::InvalidEmail {
            ("Enter the email address you use on Ko-fi.".to_owned(), true)
        } else if *error == GuildError::KofiEmailTaken {
            (error.to_string(), true)
        } else {
            (plain(&error.to_string()).to_owned(), false)
        };
        Self { email, message, on_field }
    }
}

#[page]
pub(super) async fn upgrade(cx: &Cx) -> Result<impl View> {
    let notice = take_flash(cx);

    Ok(view! { public_layout(upgrade_page(notice: notice.as_ref())) })
}

#[page(POST)]
pub(super) async fn link_kofi(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let email = pairs
        .iter()
        .find(|(name, _)| name == "email")
        .map_or_else(String::new, |(_, value)| value.clone());

    let error = match link(cx, pairs).await {
        Ok(()) => {
            set_flash(cx, FlashKind::Success, LINKED, KOFI_SECTION)?;
            return Err(see_other(format!("{PAGE}#{KOFI_SECTION}")).into());
        },
        Err(error) => error.redirect_unauthenticated()?,
    };
    let failure = KofiFailure::new(email, &error);

    Ok(view! {
        (StatusCode::UNPROCESSABLE_ENTITY)
        public_layout(upgrade_page(failure: Some(failure)))
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
    #[default] notice: Option<&Flash>,
    #[default] failure: Option<KofiFailure>,
) -> Result<impl View> {
    Ok(view! {
        <div class="page w-full mx-auto my-0 py-16 px-6">
            <div class="page-header">
                <div>
                    <h1>"Plans"</h1>
                    <p class="page-lead">
                        "Paid tiers are cost-recovery: they unlock the features that cost real money to run. Everything else stays free."
                    </p>
                </div>
            </div>
            flash(notice: notice.filter(|notice| notice.section.is_none()))
            pro_features()
            suspense(
                fallback: view! {
                    <div class="skeleton-grid">
                        shape_skeleton(shape: SkeletonShape::Panel, count: 2)
                    </div>
                },
                plan_ladder()
            )
            kofi_form(
                notice: notice.filter(|notice| notice.is_for(KOFI_SECTION)),
                failure: failure.as_ref()
            )
        </div>
    }
    .boxed())
}

#[component]
async fn pro_features() -> Result<impl View> {
    Ok(view! {
        <ul
            class="list-none m-0 mb-8 p-0 grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4"
        >
            <li class="pro-feature">
                icon(name: Icon::Music)
                <strong>"Music 24/7"</strong>
                <span>"Keep the bot connected with stay-connected and autoplay."</span>
            </li>
            <li class="pro-feature">
                icon(name: Icon::Sparkles)
                <strong>"AI replies"</strong>
                <span>"High-quality model responses instead of the free tier."</span>
            </li>
            <li class="pro-feature">
                icon(name: Icon::Gauge)
                <strong>"Faster and longer Palworld uploads"</strong>
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
    })
}

#[component]
async fn kofi_form(
    notice: Option<&Flash>,
    failure: Option<&KofiFailure>,
) -> Result<impl View> {
    let field_error =
        failure.filter(|failure| failure.on_field).map(|f| f.message.as_str());
    let summary = failure.map(|failure| failure.message.as_str());
    let email = failure.map(|failure| failure.email.as_str());

    Ok(view! {
        <section class="card" id=(KOFI_SECTION) aria-labelledby="kofi-title">
            <h2 class="label" id="kofi-title">"Link your Ko-fi email"</h2>
            <p class="page-lead">
                "Connect the email you subscribe with on Ko-fi so your paid membership follows your Discord account."
            </p>
            flash(notice: notice)
            <form method="post" action=(PAGE) data-pending="">
                if let Some(message) = summary {
                    alert(
                        variant: AlertVariant::Destructive,
                        attrs: attributes! {
                            class="error"
                            id="kofi-summary"
                            role="alert"
                            tabindex="-1"
                            autofocus=""
                        },
                        alert_title((format!("Not linked: {message}")))
                    )
                }
                field_row(
                    id: EMAIL_ID,
                    label: "Ko-fi email",
                    help: Some(EMAIL_HELP),
                    error: field_error,
                    <div class="flex flex-wrap items-center gap-2 mt-3">
                        input(
                            attrs: attributes! {
                                class="flex-1 [&]:min-w-[220px]"
                                id=(EMAIL_ID)
                                type="email"
                                name="email"
                                autocomplete="email"
                                placeholder="you@example.com"
                                required=""
                                value=(email)
                                aria-describedby=(describedby(
                                    EMAIL_ID,
                                    true,
                                    field_error.is_some(),
                                ))
                                aria-invalid=(field_error.map(|_| "true"))
                            }
                        )
                        <button
                            type="submit"
                            class=(button_variants(
                                ButtonVariant::Primary,
                                ButtonSize::Md,
                            ))
                            data-pending-label="Linking\u{2026}"
                        >
                            "Link email"
                        </button>
                    </div>
                )
            </form>
        </section>
    })
}

fn host_of(url: &str) -> Option<String> {
    Url::parse(url).ok()?.host_str().map(str::to_owned)
}

#[component]
async fn plan_ladder(cx: &Cx) -> Result<impl View> {
    let info = get_user_tier(cx).await.ok();

    Ok(view! {
        if let Some(info) = info {
            let current = info.tier.unwrap_or(Tier::Free);
            let host = info.upgrade_url.as_deref().and_then(host_of);
            <div class="plan-ladder">
                #[key(plan.css_suffix())]
                for plan in Tier::PAID_LADDER {
                    <div class=(format!("plan-card plan-{}", plan.css_suffix()))>
                        <div class="flex items-baseline justify-between gap-3">
                            <span
                                class=(class!(
                                    badge_variants(BadgeVariant::Secondary),
                                    format!("tier-{}", plan.css_suffix()),
                                ))
                            >
                                (plan.label())
                            </span>
                            <span class="text-[22px] font-bold text-text">
                                (plan.price())
                                <small class="text-[13px] font-medium text-text-2">
                                    "/mo"
                                </small>
                            </span>
                        </div>
                        <ul class="list-none m-0 p-0 flex flex-col gap-2 flex-1">
                            <li class="text-[13.5px] text-text-2">
                                <strong class="font-semibold text-text">
                                    (plan.upload_limit_mb())
                                    " MB"
                                </strong>
                                " Palworld save uploads"
                            </li>
                            <li class="text-[13.5px] text-text-2">
                                <strong class="font-semibold text-text">
                                    (plan.upload_cooldown())
                                </strong>
                                " upload cooldown"
                            </li>
                        </ul>
                        match current.cmp(&plan) {
                            Ordering::Equal => <span
                                class="py-2 px-0 rounded-control text-[13px] font-semibold text-center bg-lamp-wash text-accent"
                            >
                                "Your plan"
                            </span>,
                            Ordering::Greater => <span
                                class="py-2 px-0 rounded-control text-[13px] font-semibold text-center bg-wash-3 text-text-2"
                            >
                                "Included"
                            </span>,
                            Ordering::Less => {
                                if let Some(url) = &info.upgrade_url {
                                    <a
                                        href=(url.as_str())
                                        class=(button_variants(
                                            ButtonVariant::Primary,
                                            ButtonSize::Md,
                                        ))
                                        rel="external noopener noreferrer"
                                        target="_blank"
                                    >
                                        (host
                                            .as_ref()
                                            .map_or_else(
                                                || format!("Get {}", plan.label()),
                                                |host| format!("Get {} on {host}", plan.label()),
                                            ))
                                    </a>
                                }
                            }
                        }
                    </div>
                }
            </div>
            <div class="flex flex-wrap gap-3 mb-8">
                <p class="page-lead">
                    "Prefer to pay through Discord? Add Zayden to a server first, then subscribe from Zayden's profile in Discord."
                </p>
                <a
                    href="/invite"
                    rel="external"
                    class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))
                >
                    "Add Zayden to Discord"
                </a>
            </div>
        }
    })
}
