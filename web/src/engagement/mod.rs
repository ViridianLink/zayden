//! Engagement data: the levels leaderboard, reaction-role mappings and
//! greetings.
//!
//! Every loader and save takes the request's `cx` and authorizes first, the
//! way [`crate::auth`] does, so it answers `unauthenticated`, `forbidden`,
//! `invalid guild id` or `Zayden isn't in that server` before it reads or
//! writes anything. The exceptions are `add_greeting_channel` and
//! `remove_greeting_channel`, which parse the channel id first. Errors are
//! [`EngagementError`]s whose `Display` is the exact message the dashboard
//! shows: render them inline with
//! [`server_error_text`](crate::util::server_error_text), send
//! [`EngagementError::redirect_unauthenticated`] a signed-out visitor, and
//! answer [`EngagementError::is_invalid_form`] with 422.
//!
//! Each save reads its form with `XxxForm::from_pairs` over the
//! `Form<Vec<(String, String)>>` body: the field names are the dashboard's
//! input names, `guild` included, and an unknown, repeated or missing field is
//! refused. A route also calls `ensure_path_guild` with the guild its path
//! names, so a forged `guild` field cannot aim a save at another guild.
//!
//! The leaderboard's view state is `?scope=global&page=N`, read with
//! [`LeaderboardView::from_request`]; the bare path is this server's first
//! page.

mod display;
mod dto;
mod error;
mod form;
pub mod greetings;
mod leaderboard_view;
pub mod levels;
mod parse;
pub mod reaction_roles;

pub use display::{
    GATE_KINDS,
    TEXT_KINDS,
    channel_label,
    custom_emoji_id,
    emoji_image_url,
    message_link,
    role_label,
    unconfigured_channels,
};
pub use dto::{
    CooldownView,
    GreetingImageInfo,
    GreetingsView,
    LeaderboardEntry,
    LeaderboardPage,
    ReactionRoleInfo,
};
pub use error::EngagementError;
pub use leaderboard_view::{GLOBAL_SCOPE, LeaderboardView};
