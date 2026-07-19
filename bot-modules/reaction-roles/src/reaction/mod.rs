use serenity::all::{Http, Reaction};
use sqlx::Pool;

use crate::{ReactionRoleError, ReactionRolesManager, Result};

pub struct ReactionRoleReaction;

impl ReactionRoleReaction {
    pub async fn reaction_add<Db, Manager>(
        http: &Http,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) -> Result<()>
    where
        Db: sqlx::Database,
        Manager: ReactionRolesManager<Db>,
    {
        if reaction.member.as_ref().is_some_and(|member| member.user.bot()) {
            return Ok(());
        }

        let emoji_string = reaction.emoji.to_string();
        let reaction_role =
            Manager::row(pool, reaction.message_id, &emoji_string).await?;

        if let Some(reaction_role) = reaction_role {
            let member =
                reaction.member.as_ref().ok_or(ReactionRoleError::MissingGuildId)?;

            member
                .add_role(
                    http,
                    reaction_role.role_id(),
                    Some("User reacted to a reaction role reaction"),
                )
                .await?;
        }

        Ok(())
    }

    pub async fn reaction_remove<Db, Manager>(
        http: &Http,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) -> Result<()>
    where
        Db: sqlx::Database,
        Manager: ReactionRolesManager<Db>,
    {
        if reaction.member.as_ref().is_some_and(|member| member.user.bot()) {
            return Ok(());
        }

        let reaction_role =
            Manager::row(pool, reaction.message_id, &reaction.emoji.to_string())
                .await?;

        if let Some(reaction_role) = reaction_role {
            let user_id =
                reaction.user_id.ok_or(ReactionRoleError::MissingUserId)?;
            let member = reaction
                .guild_id
                .ok_or(ReactionRoleError::MissingGuildId)?
                .member(http, user_id)
                .await?;

            member
                .remove_role(
                    http,
                    reaction_role.role_id(),
                    Some("User removed their reaction role reaction"),
                )
                .await?;
        }

        Ok(())
    }
}
