use serenity::all::{Http, Reaction};
use sqlx::Pool;

use crate::{Error, ReactionRolesManager, Result};

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
        let emoji_string = reaction.emoji.to_string();
        let reaction_role = Manager::row(pool, reaction.message_id, &emoji_string)
            .await
            .unwrap();

        if let Some(reaction_role) = reaction_role {
            let member = reaction.member.as_ref().ok_or(Error::MissingGuildId)?;

            member
                .add_role(
                    http,
                    reaction_role.role_id(),
                    Some("User reacted to a reaction role reaction"),
                )
                .await
                .unwrap();
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
        let reaction_role = Manager::row(pool, reaction.message_id, &reaction.emoji.to_string())
            .await
            .unwrap();

        if let Some(reaction_role) = reaction_role {
            let member = reaction
                .guild_id
                .ok_or(Error::MissingGuildId)?
                .member(http, reaction.user_id.unwrap())
                .await
                .unwrap();

            member
                .remove_role(
                    http,
                    reaction_role.role_id(),
                    Some("User removed their reaction role reaction"),
                )
                .await
                .unwrap();
        }

        Ok(())
    }
}
