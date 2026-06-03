use std::borrow::Cow;
use std::collections::HashMap;

use async_trait::async_trait;
use family::commands::{
    Adopt,
    Block,
    Children,
    Marry,
    Parents,
    Partner,
    Relationship,
    ResetFamily,
    Siblings,
    Unblock,
};
use family::{FamilyManager, FamilyRow};
use futures::TryStreamExt;
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateCommand,
    CreateComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EditInteractionResponse,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::{PgPool, Postgres};
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use crate::RegistryBuilder;
use crate::registry::OverlapError;

// ===== Registry =====

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(MarryCmd)
        .add_command(AdoptCmd)
        .add_command(BlockCmd)
        .add_command(UnblockCmd)
        .add_command(ChildrenCmd)
        .add_command(ParentsCmd)
        .add_command(PartnerCmd)
        .add_command(SiblingsCmd)
        .add_command(RelationshipCmd)
        .add_command(ResetFamilyCmd)
        .add_command(TreeCmd)
        .add_component(MarryAccept)?
        .add_component(MarryDecline)?
        .add_component(AdoptAccept)?
        .add_component(AdoptDecline)?;

    Ok(())
}

pub struct FamilyTable;

#[async_trait]
impl FamilyManager<Postgres> for FamilyTable {
    async fn row(
        pool: &PgPool,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<FamilyRow>> {
        let uid: i64 = user_id.into().get().cast_signed();

        let username: Option<String> = sqlx::query_scalar!(
            "SELECT u.username FROM family f \
             JOIN users u ON u.id = f.user_id \
             WHERE f.user_id = $1",
            uid
        )
        .fetch_optional(pool)
        .await?;

        let Some(username) = username else {
            return Ok(None);
        };

        // family_partners stores (LEAST, GREATEST) so we query both columns.
        let partner_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT partner_id FROM family_partners WHERE user_id = $1 \
             UNION ALL \
             SELECT user_id FROM family_partners WHERE partner_id = $1",
            uid
        )
        .fetch(pool)
        .try_filter_map(|x| std::future::ready(Ok(x)))
        .try_collect()
        .await?;

        let parent_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT parent_id FROM family_parent_child WHERE child_id = $1",
            uid
        )
        .fetch_all(pool)
        .await?;

        let children_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT child_id FROM family_parent_child WHERE parent_id = $1",
            uid
        )
        .fetch_all(pool)
        .await?;

        let blocked_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT blocked_id FROM family_blocks WHERE user_id = $1",
            uid
        )
        .fetch_all(pool)
        .await?;

        Ok(Some(FamilyRow {
            id: uid,
            username,
            partner_ids,
            parent_ids,
            children_ids,
            blocked_ids,
        }))
    }

    async fn tree<'a>(
        pool: &PgPool,
        user_id: impl Into<UserId> + Send,
        mut tree: HashMap<i32, Vec<FamilyRow>>,
        depth: i32,
        add_parents: bool,
        add_partners: bool,
    ) -> sqlx::Result<HashMap<i32, Vec<FamilyRow>>> {
        let user_id = user_id.into();
        let signed_id = user_id.get().cast_signed();

        // Cycle prevention: skip if already in tree.
        if tree.values().flatten().any(|row| row.id == signed_id) {
            return Ok(tree);
        }

        let Some(row) = Self::row(pool, user_id).await? else {
            return Ok(tree);
        };

        let partner_ids = row.partner_ids.clone();
        let parent_ids = row.parent_ids.clone();
        let children_ids = row.children_ids.clone();

        tree.entry(depth).or_default().push(row);

        if add_parents {
            for parent_id in parent_ids {
                let pid = UserId::new(parent_id.cast_unsigned());
                tree = Box::pin(Self::tree(
                    pool,
                    pid,
                    tree,
                    depth - 1,
                    true,
                    add_partners,
                ))
                .await?;
            }
        }

        if add_partners {
            for partner_id in partner_ids {
                let pid = UserId::new(partner_id.cast_unsigned());
                // Don't recurse into partners' partners to prevent runaway
                // expansion.
                tree = Box::pin(Self::tree(pool, pid, tree, depth, false, false))
                    .await?;
            }
        }

        for child_id in children_ids {
            let cid = UserId::new(child_id.cast_unsigned());
            tree = Box::pin(Self::tree(
                pool,
                cid,
                tree,
                depth + 1,
                false,
                add_partners,
            ))
            .await?;
        }

        Ok(tree)
    }

    async fn reset(pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM family").execute(pool).await?;
        Ok(())
    }

    async fn save(pool: &PgPool, row: &FamilyRow) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username",
            row.id,
            &row.username
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO family (user_id) VALUES ($1) ON CONFLICT DO NOTHING",
            row.id
        )
        .execute(pool)
        .await?;

        // Sync partners. The schema enforces user_id < partner_id via CHECK.
        for &partner_id in &row.partner_ids {
            ensure_family_user(pool, partner_id).await?;
            let (uid, pid) = if row.id < partner_id {
                (row.id, partner_id)
            } else {
                (partner_id, row.id)
            };
            sqlx::query!(
                "INSERT INTO family_partners (user_id, partner_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
                uid,
                pid
            )
            .execute(pool)
            .await?;
        }

        // Sync children (this user is the parent).
        for &child_id in &row.children_ids {
            ensure_family_user(pool, child_id).await?;
            sqlx::query!(
                "INSERT INTO family_parent_child (parent_id, child_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING", row.id, child_id
            )
            .execute(pool)
            .await?;
        }

        // Sync parents (this user is the child).
        for &parent_id in &row.parent_ids {
            ensure_family_user(pool, parent_id).await?;
            sqlx::query!(
                "INSERT INTO family_parent_child (parent_id, child_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING", parent_id, row.id
            )
            .execute(pool)
            .await?;
        }

        // Sync blocked users.
        for &blocked_id in &row.blocked_ids {
            ensure_family_user(pool, blocked_id).await?;
            sqlx::query!(
                "INSERT INTO family_blocks (user_id, blocked_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
                row.id,
                blocked_id
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }
}

async fn ensure_family_user(pool: &PgPool, id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, 'Unknown') ON CONFLICT DO NOTHING", id
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO family (user_id) VALUES ($1) ON CONFLICT DO NOTHING",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn proposal_buttons<'a>(
    accept_id: &'a str,
    decline_id: &'a str,
) -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new(accept_id).label("Accept").style(ButtonStyle::Success),
        CreateButton::new(decline_id).label("Decline").style(ButtonStyle::Danger),
    ]))
}

pub struct MarryCmd;

#[async_trait]
impl ModuleCommand for MarryCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("marry")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Marry::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let target_id =
            Marry::run::<Postgres, FamilyTable>(cx.ctx, cx.interaction, &cx.app.db)
                .await?;

        let content = format!(
            "{}, {} wants to marry you! Do you accept?",
            target_id.mention(),
            cx.interaction.user.mention()
        );

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![proposal_buttons(
                            "marry_accept",
                            "marry_decline",
                        )]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptCmd;

#[async_trait]
impl ModuleCommand for AdoptCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("adopt")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Adopt::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let target_id =
            Adopt::run::<Postgres, FamilyTable>(cx.ctx, cx.interaction, &cx.app.db)
                .await?;

        let content = format!(
            "{}, {} wants to adopt you! Do you accept?",
            target_id.mention(),
            cx.interaction.user.mention()
        );

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(vec![proposal_buttons(
                            "adopt_accept",
                            "adopt_decline",
                        )]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct BlockCmd;

#[async_trait]
impl ModuleCommand for BlockCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("block")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Block::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Block::run::<Postgres, FamilyTable>(cx.ctx, cx.interaction, &cx.app.db)
            .await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("User blocked.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct UnblockCmd;

#[async_trait]
impl ModuleCommand for UnblockCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("unblock")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Unblock::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Unblock::run::<Postgres, FamilyTable>(cx.ctx, cx.interaction, &cx.app.db)
            .await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("User unblocked.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct ChildrenCmd;

#[async_trait]
impl ModuleCommand for ChildrenCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("children")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Children::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) = Children::run::<Postgres, FamilyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        let content =
            format!("{}'s children: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct ParentsCmd;

#[async_trait]
impl ModuleCommand for ParentsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("parents")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Parents::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) = Parents::run::<Postgres, FamilyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        let content =
            format!("{}'s parents: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct PartnerCmd;

#[async_trait]
impl ModuleCommand for PartnerCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("partner")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Partner::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) = Partner::run::<Postgres, FamilyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        let content =
            format!("{}'s partners: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct SiblingsCmd;

#[async_trait]
impl ModuleCommand for SiblingsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("siblings")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Siblings::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let (user_id, names) = Siblings::run::<Postgres, FamilyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        let content =
            format!("{}'s siblings: {}", user_id.mention(), names.join(", "));

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct RelationshipCmd;

#[async_trait]
impl ModuleCommand for RelationshipCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("relationship")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Relationship::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        // Relationship::run defers the interaction internally.
        let resp = Relationship::run::<Postgres, FamilyTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        let content = format!(
            "{} and {} are: **{}**",
            resp.user_id.mention(),
            resp.other_id.mention(),
            resp.relationship
        );

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

pub struct ResetFamilyCmd;

#[async_trait]
impl ModuleCommand for ResetFamilyCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("resetfamily")
    }

    fn definition(&self) -> CreateCommand<'static> {
        ResetFamily::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        ResetFamily::run::<Postgres, FamilyTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Family trees have been reset.")
                        .ephemeral(true),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct TreeCmd;

#[async_trait]
impl ModuleCommand for TreeCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("tree")
    }

    fn definition(&self) -> CreateCommand<'static> {
        family::commands::Tree::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let user = match cx.interaction.data.options().first() {
            Some(ResolvedOption { value: ResolvedValue::User(user, _), .. }) => {
                *user
            },
            _ => &cx.interaction.user,
        };

        let row = FamilyTable::row(&cx.app.db, user.id)
            .await?
            .unwrap_or_else(|| user.into());

        let tree = row.tree::<Postgres, FamilyTable>(&cx.app.db).await?;

        let content = format_tree(&tree, user.id);

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content(content),
            )
            .await?;

        Ok(())
    }
}

fn format_tree(tree: &HashMap<i32, Vec<FamilyRow>>, root_id: UserId) -> String {
    if tree.is_empty() {
        return "Your family tree is empty.".to_string();
    }

    let mut keys: Vec<i32> = tree.keys().copied().collect();
    keys.sort_unstable();

    let root_signed = root_id.get().cast_signed();
    let mut lines = Vec::new();

    for depth in keys {
        let members =
            tree.get(&depth).expect("key from tree.keys() is always present");
        let prefix = match depth.cmp(&0) {
            std::cmp::Ordering::Less => "⬆",
            std::cmp::Ordering::Greater => "⬇",
            std::cmp::Ordering::Equal => "◆",
        };
        let names: Vec<&str> = members.iter().map(|r| r.username.as_str()).collect();

        let label = if depth == 0 {
            let root_name = members
                .iter()
                .find(|r| r.id == root_signed)
                .map_or("You", |r| r.username.as_str());
            format!("{prefix} **{root_name}** (+ partners)")
        } else {
            format!("{prefix} {}", names.join(", "))
        };

        lines.push(label);
    }

    let text = lines.join("\n");
    // Discord message limit is 2000 characters.
    if text.len() > 1990 {
        let truncated: String = text.chars().take(1997).collect();
        format!("{truncated}...")
    } else {
        text
    }
}

pub struct MarryAccept;

#[async_trait]
impl ModuleComponent for MarryAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("marry_accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::marry::accept::<Postgres, FamilyTable>(
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Congratulations! You are now married!")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct MarryDecline;

#[async_trait]
impl ModuleComponent for MarryDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("marry_decline"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::marry::decline(cx.interaction)?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Marriage proposal declined.")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptAccept;

#[async_trait]
impl ModuleComponent for AdoptAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("adopt_accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::adopt::accept::<Postgres, FamilyTable>(
            cx.interaction,
            &cx.app.db,
        )
        .await?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Adoption accepted! Welcome to the family!")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}

pub struct AdoptDecline;

#[async_trait]
impl ModuleComponent for AdoptDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("adopt_decline"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        family::components::adopt::decline(cx.interaction)?;

        cx.interaction
            .create_response(
                &cx.ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Adoption declined.")
                        .components(vec![]),
                ),
            )
            .await?;
        Ok(())
    }
}
