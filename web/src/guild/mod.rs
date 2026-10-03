//! Guild data: the guild list and switcher, settings sections and their
//! saves, module toggles, Patreon and YouTube connections, tiers, Ko-fi links
//! and slash-command permissions.
//!
//! Every loader and save takes the request's `cx`. Guild-scoped calls
//! authorize first, the way [`crate::auth`] does: they answer
//! `unauthenticated`, `forbidden`, `invalid guild id` or `Zayden isn't in that
//! server` before they read or write anything. The exceptions are
//! `link_kofi_email` (validates the email first), `set_module_enabled`
//! (rejects an unknown module first), `get_user_tier` (`Ok` when signed out)
//! and `guild_server_tier` (no access check). Errors are [`GuildError`]s whose
//! `Display` is the exact message the dashboard shows; render them inline with
//! [`server_error_text`](crate::util::server_error_text), send
//! [`GuildError::redirect_unauthenticated`] a signed-out visitor, and answer
//! [`GuildError::is_invalid_form`] with 422.
//!
//! Each save reads its form with `XxxForm::from_pairs` over the
//! `Form<Vec<(String, String)>>` body: the field names are the dashboard's input
//! names, `guild` included, and an unknown, repeated or missing field is
//! refused.

mod access;
pub mod command_permissions;
pub mod directory;
pub mod dto;
mod error;
pub mod faq;
mod form;
pub mod kofi;
pub mod modules;
pub mod parse;
pub mod patreon;
pub mod sections;
pub mod settings;
pub mod support;
pub mod tier;
pub mod youtube;

pub use access::admin_app;
pub use directory::{get_active_guild, get_guild_directory, list_manageable_guilds};
pub use error::GuildError;
pub use form::GuildForm;
pub use sections::get_section_settings;
