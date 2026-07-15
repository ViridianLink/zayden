use leptos::prelude::*;

use crate::dto::Tier;
use crate::server::tier::get_user_tier;

#[component]
pub(crate) fn TierBadge() -> impl IntoView {
    let tier_info = Resource::new(|| (), |()| get_user_tier());

    view! {
        <Suspense fallback=|| ()>
            {move || {
                tier_info
                    .get()
                    .and_then(Result::ok)
                    .and_then(|info| info.tier.map(|t| (t, info.upgrade_url)))
                    .map(|(tier, upgrade_url)| {
                        let tier_cls = format!("tier-badge tier-{}", tier.css_suffix());
                        let is_free = tier == Tier::Free;
                        view! {
                            <span class=tier_cls>{tier.label()}</span>
                            {upgrade_url.filter(|_| is_free).map(|url| view! {
                                <a
                                    href=url
                                    class="btn-upgrade"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >"Upgrade to Pro"</a>
                            })}
                        }
                        .into_any()
                    })
            }}
        </Suspense>
    }
}
