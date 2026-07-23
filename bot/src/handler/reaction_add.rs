use reaction_roles::ReactionRoleReaction;
use serenity::all::{Context, Reaction};
use sqlx::{PgPool, Postgres};
use suggestions::{GuildTable, Suggestions};

use super::Handler;
use crate::Result;

impl Handler {
    pub(super) async fn reaction_add(
        ctx: &Context,
        reaction: &Reaction,
        pool: &PgPool,
    ) -> Result<()> {
        ReactionRoleReaction::reaction_add(&ctx.http, reaction, pool).await?;

        Suggestions::reaction::<Postgres, GuildTable>(&ctx.http, reaction, pool)
            .await?;

        Ok(())
    }
}
