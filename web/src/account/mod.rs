pub mod deadlock;
mod page;
mod steam;

use topcoat::router::RouterBuilder;

pub const ACCOUNT_TITLE: &str = "Linked accounts - Zayden";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(page::account)
        .page(page::unlink)
        .page(deadlock::save_dm)
        .page(deadlock::save_servers)
        .route(steam::steam_link)
        .route(steam::steam_callback)
}
