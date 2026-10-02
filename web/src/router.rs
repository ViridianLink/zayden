use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{document, pages};

/// Registers every layout, page and route on `base`.
///
/// `base` carries what the handlers read from app context: the asset
/// configuration and, in the server binary, the
/// [`WebState`](crate::state::WebState).
#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    base.layout(document::root_layout)
        .page(pages::not_found::not_found)
        .runtime()
        .build()
}
