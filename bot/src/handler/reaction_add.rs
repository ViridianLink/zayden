use reaction_roles::ReactionRoleReaction;
use serenity::all::{Context, Reaction};
use sqlx::{PgPool, Postgres};
use suggestions::Suggestions;

use crate::Result;
use crate::modules::reaction_roles::ReactionRolesTable;
use crate::sqlx_lib::GuildTable;

use super::Handler;

impl Handler {
    pub(super) async fn reaction_add(
        ctx: &Context,
        reaction: &Reaction,
        pool: &PgPool,
    ) -> Result<()> {
        ReactionRoleReaction::reaction_add::<Postgres, ReactionRolesTable>(
            &ctx.http, reaction, pool,
        )
        .await?;
        Suggestions::reaction::<Postgres, GuildTable>(&ctx.http, reaction, pool).await;

        Ok(())
    }
}
