use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;

use crate::dto::Tier;
use crate::server::kofi::LinkKofiEmail;
use crate::server::tier::get_user_tier;
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;

#[component]
pub(crate) fn UpgradePage() -> impl IntoView {
    let tier_info = Resource::new(|| (), |()| get_user_tier());
    let link = ServerAction::<LinkKofiEmail>::new();

    view! {
        <Title text="Upgrade - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Upgrade your plan"</h1>
                        <p class="page-lead">
                            "Paid tiers are cost-recovery: they unlock the features that cost "
                            "real money to run. Everything else stays free."
                        </p>
                    </div>
                </div>

                <ul class="pro-features">
                    <li class="pro-feature">
                        <Icon name="music"/>
                        <strong>"Music 24/7"</strong>
                        <span>"Keep the bot connected with stay-connected and autoplay."</span>
                    </li>
                    <li class="pro-feature">
                        <Icon name="sparkles"/>
                        <strong>"AI replies"</strong>
                        <span>"High-quality model responses instead of the free tier."</span>
                    </li>
                    <li class="pro-feature">
                        <Icon name="gauge"/>
                        <strong>"Faster & longer Palworld uploads"</strong>
                        <span>"Shorter upload cooldown and larger save quota."</span>
                    </li>
                </ul>

                <Suspense fallback=|| ()>
                    {move || tier_info.get().and_then(Result::ok).map(|info| {
                        let current = info.tier.unwrap_or(Tier::Free);
                        let upgrade_url = info.upgrade_url;
                        let cards = Tier::PAID_LADDER.into_iter().map(|plan| {
                            let cta = match current.cmp(&plan) {
                                std::cmp::Ordering::Equal => {
                                    view! { <span class="plan-current">"Your plan"</span> }
                                        .into_any()
                                },
                                std::cmp::Ordering::Greater => {
                                    view! { <span class="plan-included">"Included"</span> }
                                        .into_any()
                                },
                                std::cmp::Ordering::Less => {
                                    upgrade_url.clone().map(|url| view! {
                                        <a
                                            href=url
                                            class="btn btn-primary"
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >{format!("Get {}", plan.label())}</a>
                                    }).into_any()
                                },
                            };
                            view! {
                                <div class=format!("plan-card plan-{}", plan.css_suffix())>
                                    <div class="plan-head">
                                        <span class=format!(
                                            "tier-badge tier-{}", plan.css_suffix(),
                                        )>{plan.label()}</span>
                                        <span class="plan-price">
                                            {plan.price()}<small>"/mo"</small>
                                        </span>
                                    </div>
                                    <ul class="plan-specs">
                                        <li>
                                            <strong>{plan.upload_limit_mb()}" MB"</strong>
                                            " Palworld save uploads"
                                        </li>
                                        <li>
                                            <strong>{plan.upload_cooldown()}</strong>
                                            " upload cooldown"
                                        </li>
                                    </ul>
                                    {cta}
                                </div>
                            }
                        }).collect_view();
                        view! {
                            <div class="plan-ladder">{cards}</div>
                            <div class="upgrade-actions">
                                <a href="/invite" rel="external" class="btn btn-secondary">
                                    "Subscribe via Discord"
                                </a>
                            </div>
                        }.into_any()
                    })}
                </Suspense>

                <div class="card">
                    <p class="label">"Link your Ko-fi email"</p>
                    <p class="page-lead">
                        "Connect the email you subscribe with on Ko-fi so your paid "
                        "membership follows your Discord account."
                    </p>
                    <ActionForm action=link>
                        <div class="kofi-link-form">
                            <input
                                type="email"
                                name="email"
                                placeholder="you@example.com"
                                required=true
                            />
                            <button type="submit" class="btn btn-primary">"Link email"</button>
                        </div>
                    </ActionForm>
                    {move || link.value().get().map(|r| match r {
                        Ok(()) => view! {
                            <p class="success">"Ko-fi email linked."</p>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="error">{e.to_string()}</p>
                        }.into_any(),
                    })}
                </div>
            </div>
        </AppShell>
    }
}
