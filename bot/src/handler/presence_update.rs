use serenity::all::{Context, Presence};

use crate::Result;

use super::Handler;

impl Handler {
    pub(super) async fn presence_update(&self, ctx: &Context, new: &Presence) -> Result<()> {
        llamad2::StatusUpdate::presence_update(ctx, new).await;

        Ok(())
    }
}
