//! Operator and Destiny 2 loadout administration: role checks, the bot's
//! server list, loadout reads and writes, and Zayden emoji uploads.
//!
//! Every call takes the request `cx` and authorizes itself, so pages render
//! with HTTP 200 and show refusals inline. Errors are [`AdminError`] (or
//! [`AuthError`](crate::auth::AuthError) for the role checks), and their
//! `Display` is the bare message. Pages that show a load, save or delete
//! failure prefix it with [`server_error_text`](crate::util::server_error_text);
//! the emoji-create panel shows [`create_zayden_emoji`]'s error as is, and
//! [`LoadoutCheck::error`] is never prefixed. Use [`AdminError::is_denied`] to
//! choose the "access required" copy over the generic error copy.
//!
//! - [`is_admin`], [`is_operator`] and [`guild_operator_access`] are `false` for a
//!   signed-out visitor.
//! - [`list_bot_guilds`] needs the `operator` role; every loadout and emoji call
//!   needs `admin`.
//! - A flat form post folds into a [`LoadoutForm`] with [`fold_loadout_form`].
//! - The pool-level helpers ([`summaries`], [`stored_form`], [`catalog`],
//!   [`write_unchecked`], [`remove_unchecked`]) skip the role check; callers must
//!   have passed `require_role(cx, WebRole::Admin)`.

mod access;
pub mod convert;
pub mod dto;
mod emoji;
pub mod emoji_upload;
mod error;
pub mod fields;
pub mod keys;
mod loadouts;
mod operator;

pub use access::{guild_operator_access, is_admin, is_operator};
pub use convert::{
    blank,
    editor_form,
    loadout_check,
    loadout_form,
    options,
    raw_loadout,
    reserved_keys,
};
pub use dto::{
    ArmourForm,
    ArmourPieceInfo,
    AspectForm,
    CatalogWeaponInfo,
    EmojiInfo,
    EmojiSource,
    LoadoutCatalog,
    LoadoutCheck,
    LoadoutForm,
    LoadoutOptions,
    LoadoutSummary,
    StatForm,
    UsageInfo,
    WeaponForm,
};
pub use emoji::{create_zayden_emoji, emoji_image, zayden_emojis};
pub use error::{AdminError, EmojiUploadError, LoadoutFieldError, LoadoutFormError};
pub use fields::{fold_loadout_form, loadout_pairs, row_name};
pub use loadouts::{
    catalog,
    check_loadout,
    delete_loadout,
    draft,
    get_loadout,
    list_loadouts,
    loadout_catalog,
    remove_unchecked,
    save_loadout,
    stored_form,
    summaries,
    write_unchecked,
};
pub use operator::{list_bot_guilds, parse_guild_id};
