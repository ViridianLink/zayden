use serenity::all::{ActivityData, Context, OnlineStatus, Presence};
use tracing::warn;

use crate::{LLAMA_GUILD, LLAMA_USER};

const LLAMA_ONLINE_STATUS: &str = "!help";
const LLAMA_OFFLINE_STATUS: &str = "I'm the King now!";

pub struct StatusUpdate;

impl StatusUpdate {
    pub fn on_ready(ctx: &Context) {
        ctx.set_activity(Some(ActivityData::custom(LLAMA_ONLINE_STATUS)));
    }

    pub fn presence_update(ctx: &Context, new: &Presence) {
        if new.user.id != LLAMA_USER
            || new.guild_id.is_none_or(|id| id != LLAMA_GUILD)
        {
            return;
        }

        match new.status {
            OnlineStatus::DoNotDisturb
            | OnlineStatus::Idle
            | OnlineStatus::Online => {
                ctx.set_activity(Some(ActivityData::custom(LLAMA_ONLINE_STATUS)));
            },
            OnlineStatus::Offline | OnlineStatus::Invisible => {
                ctx.set_activity(Some(ActivityData::custom(LLAMA_OFFLINE_STATUS)));
            },
            other => {
                warn!(status = ?other, "llamad2 presence_update: unhandled OnlineStatus variant");
            },
        }
    }
}
