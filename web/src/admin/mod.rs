mod access;
pub mod convert;
pub mod dto;
pub mod editor;
mod emoji;
pub mod emoji_upload;
mod error;
pub mod fields;
pub mod keys;
mod loadouts;
mod operator;
pub mod pages;

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
