#![cfg_attr(
    not(feature = "ssr"),
    expect(
        clippy::unused_async_trait_impl,
        reason = "Leptos #[server] macro generates an unawaited `run_body` stub for non-ssr builds"
    )
)]
#![expect(
    clippy::must_use_candidate,
    reason = "Leptos #[component] functions return `impl IntoView` which is consumed by the view macro, never a must_use value"
)]
#![allow(
    clippy::empty_enums,
    reason = "Leptos #[component] derives TypedBuilder, which generates an uninhabited marker enum per component. `allow`, not `expect`: the lint fires on nightly but not on the stable clippy CI runs, so an expectation is unfulfilled there"
)]

pub mod app;
pub mod dto;
pub mod server;
pub mod ui;
#[cfg(feature = "ssr")]
pub mod util;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn hydrate() {
    leptos::mount::hydrate_body(app::App);
}
