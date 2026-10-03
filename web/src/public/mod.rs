mod invite;
mod landing;
pub mod layout;
mod legal;
mod login;
mod privacy;
mod terms;

use topcoat::router::RouterBuilder;

pub const LANDING_TITLE: &str = "Zayden — the all-in-one Discord bot";
pub const LOGIN_TITLE: &str = "Sign In - Zayden Dashboard";
pub const PRIVACY_TITLE: &str = "Privacy Policy - Zayden";
pub const TERMS_TITLE: &str = "Terms of Service - Zayden";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(landing::landing)
        .page(login::login)
        .page(privacy::privacy)
        .page(terms::terms)
        .route(invite::invite)
}
