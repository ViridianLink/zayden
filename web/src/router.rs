use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::font::RouterBuilderFontExt;
use topcoat::router::RouterBuilder;
use topcoat::runtime::{PrefetchMode, RouterBuilderRuntimeExt};

use crate::app::{self, providers};
use crate::font::GEIST;

#[must_use]
pub fn router() -> RouterBuilder {
    app::routes()
        .font(GEIST)
        .origin_policy(providers::origin_policy())
        .runtime()
        .prefetch(PrefetchMode::Never)
        .cookies()
}
