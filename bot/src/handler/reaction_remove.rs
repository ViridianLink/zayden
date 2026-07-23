use reaction_roles::ReactionRoleReaction;
use serenity::all::{Context, Reaction};
use sqlx::PgPool;
use suggestions::Suggestions;

use super::Handler;
use crate::Result;

impl Handler {
    pub(super) async fn reaction_remove(
        ctx: &Context,
        reaction: &Reaction,
        pool: &PgPool,
    ) -> Result<()> {
        ReactionRoleReaction::reaction_remove(&ctx.http, reaction, pool).await?;

        Suggestions::reaction(&ctx.http, reaction, pool).await?;

        Ok(())
    }
}
