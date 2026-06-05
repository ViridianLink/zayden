use std::collections::HashMap;
use std::fmt::Write as _;

use serenity::all::{ResolvedOption, ResolvedValue};

pub mod cache;
pub use cache::{EmojiCache, EmojiCacheData, EmojiResult, GuildMembersCache};

pub mod cron;
pub use cron::{ActionFn, CronJob, CronJobData};

pub mod modals;
pub use modals::{parse_modal_components, parse_text_components};

pub mod templates;

pub mod error;
pub use error::{CoreError as Error, HandlerError, Respond};

pub mod events;
pub mod format_num;
pub use format_num::FormatNum;

pub mod scope;
pub use scope::{CommandMetadata, CommandScope, IdMatch};

pub mod snowflake;
pub use snowflake::{as_i64, as_u64};

pub mod ctx;
pub use ctx::{AutocompleteCtx, ComponentCtx, InvocationCtx, ModalCtx};

pub mod module;
pub use module::{ModuleAutocomplete, ModuleCommand, ModuleComponent, ModuleModal};
use tracing::warn;

pub fn parse_options<'a>(
    options: impl IntoIterator<Item = ResolvedOption<'a>>,
) -> HashMap<&'a str, ResolvedValue<'a>> {
    options.into_iter().map(|option| (option.name, option.value)).collect()
}

pub fn parse_options_ref<'a>(
    options: impl IntoIterator<Item = &'a ResolvedOption<'a>>,
) -> HashMap<&'a str, &'a ResolvedValue<'a>> {
    options.into_iter().map(|option| (option.name, &option.value)).collect()
}

#[must_use]
pub fn get_option_str(options: &[ResolvedOption<'_>]) -> String {
    let mut s = String::new();

    for option in options {
        s.push(' ');
        s.push_str(option.name);

        if !matches!(
            option.value,
            ResolvedValue::SubCommandGroup(_) | ResolvedValue::SubCommand(_)
        ) {
            s.push_str(": ");
        }

        match &option.value {
            ResolvedValue::SubCommandGroup(sub_options)
            | ResolvedValue::SubCommand(sub_options) => {
                s.push_str(&get_option_str(sub_options));
            },
            ResolvedValue::User(user, _) => {
                let _ = write!(s, "User({{id: {}, name: {}}})", user.id, user.name);
            },
            ResolvedValue::Autocomplete { .. }
            | ResolvedValue::Boolean(_)
            | ResolvedValue::Integer(_)
            | ResolvedValue::Number(_)
            | ResolvedValue::String(_)
            | ResolvedValue::Attachment(_)
            | ResolvedValue::Channel(_)
            | ResolvedValue::Role(_)
            | ResolvedValue::Unresolved(_) => {
                let _ = write!(s, "{:?}", option.value);
            },
            _ => {
                warn!("unexpected resolved option type: {:?}", option.value);
                let _ = write!(s, "{:?}", option.value);
            },
        }
    }

    s
}
