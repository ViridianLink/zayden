use serenity::all::{Context, Presence};

use super::Handler;

impl Handler {
    pub(super) fn presence_update(ctx: &Context, new: &Presence) {
        llamad2::StatusUpdate::presence_update(ctx, new);
    }
}
