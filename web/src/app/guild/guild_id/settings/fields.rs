use twilight_model::channel::ChannelType;

use super::Lists;
use crate::components::pickers::{
    SelectOption,
    channel_options,
    forum_tag_options,
    role_options,
};

const CHANNELS_UNAVAILABLE: &str = "Couldn't reach Discord, so the channel list \
                                    is unavailable. Saving keeps the current \
                                    value.";

const ROLES_UNAVAILABLE: &str = "Couldn't reach Discord, so the role list is \
                                 unavailable. Saving keeps the current value.";

pub(super) fn channels(
    lists: &Lists,
    kinds: &[ChannelType],
) -> Result<Vec<SelectOption>, &'static str> {
    lists
        .channels()
        .ok()
        .and_then(|channels| channel_options(Ok(channels), kinds).ok())
        .ok_or(CHANNELS_UNAVAILABLE)
}

pub(super) fn roles(lists: &Lists) -> Result<Vec<SelectOption>, &'static str> {
    lists
        .roles()
        .ok()
        .and_then(|roles| role_options(Ok(roles)).ok())
        .ok_or(ROLES_UNAVAILABLE)
}

pub(super) fn forum_tags(lists: &Lists) -> Result<Vec<SelectOption>, &'static str> {
    lists
        .channels()
        .ok()
        .and_then(|channels| forum_tag_options(Ok(channels)).ok())
        .ok_or(CHANNELS_UNAVAILABLE)
}
