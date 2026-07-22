use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::{Database, Pool};
use zayden_core::{as_i64, as_u64};

use crate::family_manager::{FamilyManager, FamilyRow};
use crate::{FamilyError, Result};

pub struct Siblings;

impl Siblings {
    pub async fn run<Db: Database, Manager: FamilyManager<Db>>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<(UserId, Vec<String>)> {
        let user = match interaction.data.options().first() {
            Some(ResolvedOption { value: ResolvedValue::User(user, _), .. }) => {
                *user
            },
            _ => &interaction.user,
        };

        let guild_id = interaction.guild_id.ok_or(FamilyError::MissingGuildId)?;

        let row = Manager::row(pool, guild_id, user.id)
            .await?
            .unwrap_or_else(|| FamilyRow::from_user(guild_id, user));

        if row.parent_ids.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoSiblings);
            }
            return Err(FamilyError::NoSiblings(user.id));
        }

        let user_id_signed: i64 = as_i64(user.id.get());

        let mut parent_rows = Vec::with_capacity(row.parent_ids.len());
        for parent_id in row.parent_ids {
            let parent_uid = UserId::new(as_u64(parent_id));
            if let Some(parent_row) =
                Manager::row(pool, guild_id, parent_uid).await?
            {
                parent_rows.push(parent_row);
            }
        }

        let mut siblings = Vec::new();
        for sib_id in collect_sibling_ids(&parent_rows, user_id_signed) {
            let sib_uid = UserId::new(as_u64(sib_id));
            let sib_user = sib_uid.to_user(ctx).await?;
            siblings.push(sib_user.mention().to_string());
        }

        if siblings.is_empty() {
            if user == &interaction.user {
                return Err(FamilyError::SelfNoSiblings);
            }
            return Err(FamilyError::NoSiblings(user.id));
        }

        Ok((user.id, siblings))
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("siblings")
            .description("List who your siblings are.")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to check. Leave blank to check yourself.",
            ))
    }
}

fn collect_sibling_ids(parent_rows: &[FamilyRow], user_id: i64) -> Vec<i64> {
    parent_rows
        .iter()
        .flat_map(|row| row.children_ids.iter().copied())
        .filter(|id| *id != user_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_with_children(id: i64, children: Vec<i64>) -> FamilyRow {
        FamilyRow { id, children_ids: children, ..Default::default() }
    }

    #[test]
    fn collect_sibling_ids_combines_children_from_multiple_parents() {
        let user_id = 1;
        let parent_a = row_with_children(10, vec![user_id, 2]);
        let parent_b = row_with_children(20, vec![user_id, 3]);

        let siblings = collect_sibling_ids(&[parent_a, parent_b], user_id);

        assert_eq!(siblings, vec![2, 3]);
    }

    #[test]
    fn collect_sibling_ids_excludes_only_self() {
        let user_id = 1;
        let parent = row_with_children(10, vec![user_id]);

        let siblings = collect_sibling_ids(&[parent], user_id);

        assert!(siblings.is_empty());
    }
}
