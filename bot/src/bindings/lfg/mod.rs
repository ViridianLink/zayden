mod slash_command;

use std::borrow::Cow;

use async_trait::async_trait;
use jiff::tz;
use jiff::tz::TimeZone;
use jiff_sqlx::Timestamp;
use lfg::commands::{JoinedManager, SetupManager};
use lfg::components::{EditManager, EditRow};
use lfg::models::timezone_manager::locale_to_timezone;
use lfg::{
    Components,
    GuildTable,
    Join,
    JoinedRow,
    KickComponent,
    LfgError,
    PostManager,
    PostRow,
    Savable,
    TagsComponent,
    TimezoneManager,
};
use serenity::all::{GenericChannelId, GuildId, MessageId, RoleId, UserId};
pub use slash_command::Lfg;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;
use zayden_core::{as_i64, as_u64};

use crate::registry::OverlapError;
use crate::{BotState, RegistryBuilder};

// region: PostTable

pub struct PostTable;

#[async_trait]
impl PostManager<Postgres> for PostTable {
    async fn exists(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<bool> {
        let id = id.into();

        sqlx::query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM lfg_posts WHERE id = $1)",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
        .map(|exists| exists.unwrap_or(false))
    }

    async fn owner(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<UserId> {
        let id = id.into();

        sqlx::query_scalar!(
            "SELECT owner_id from lfg_posts WHERE id = $1",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
        .map(|id| UserId::new(as_u64(id)))
    }

    async fn post_row(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<PostRow> {
        let id = id.into();

        sqlx::query_file_as!(
            PostRow,
            "sql/lfg/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(pool)
        .await
    }

    async fn join(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
        user: impl Into<UserId> + Send,
        alternative: bool,
    ) -> Result<PostRow, LfgError> {
        let id = id.into();
        let user = user.into();

        let mut tx = pool.begin().await?;

        sqlx::query_file_as!(
            PostRow,
            "sql/lfg/PostManager/join.sql",
            as_i64(id.get()),
            as_i64(user.get()),
            alternative
        )
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_file_as!(
            PostRow,
            "sql/lfg/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(&mut *tx)
        .await?;

        if !alternative && row.fireteam_len() > row.fireteam_size() {
            return Err(LfgError::FireteamFull);
        }

        tx.commit().await?;

        Ok(row)
    }

    async fn leave(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
        user: impl Into<UserId> + Send,
    ) -> sqlx::Result<PostRow> {
        let id = id.into();
        let user = user.into();

        let mut tx = pool.begin().await?;

        sqlx::query_file_as!(
            PostRow,
            "sql/lfg/PostManager/leave.sql",
            as_i64(id.get()),
            as_i64(user.get()),
        )
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query_file_as!(
            PostRow,
            "sql/lfg/PostManager/post_row.sql",
            as_i64(id.get())
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(row)
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
    )]
    async fn edit(pool: &PgPool, post: &PostRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "sql/lfg/PostManager/edit.sql",
            post.id,
            post.owner_id,
            post.activity,
            post.start_time as Timestamp,
            post.description,
            post.fireteam_size,
        )
        .execute(pool)
        .await
    }

    async fn delete(
        pool: &PgPool,
        id: impl Into<GenericChannelId> + Send,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query!("DELETE FROM lfg_posts WHERE id = $1", as_i64(id.get()))
            .execute(pool)
            .await
    }
}

#[expect(
    trivial_casts,
    reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
)]
async fn save_post(pool: &PgPool, row: PostRow) -> sqlx::Result<PgQueryResult> {
    let mut tx = pool.begin().await?;

    let main_result = sqlx::query_file!(
        "sql/lfg/PostManager/save_post.sql",
        row.id,
        row.owner_id,
        row.activity,
        row.start_time as Timestamp,
        row.description,
        row.fireteam_size,
        &row.fireteam,
        &row.alternatives
    )
    .execute(&mut *tx)
    .await?;

    if let (Some(channel), Some(message)) = (row.alt_channel, row.alt_message) {
        sqlx::query!(
            "INSERT INTO lfg_messages (post_id, message_id, channel_id) VALUES ($1, $2, $3) ON CONFLICT (post_id) DO NOTHING",
            row.id,
            message,
            channel,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(main_result)
}

#[async_trait]
impl Savable<Postgres, PostRow> for PostTable {
    async fn save(pool: &PgPool, row: PostRow) -> sqlx::Result<PgQueryResult> {
        save_post(pool, row).await
    }
}

#[async_trait]
impl SetupManager<Postgres> for PostTable {
    async fn insert(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
        channel: impl Into<GenericChannelId> + Send,
        role: Option<RoleId>,
    ) -> sqlx::Result<PgQueryResult> {
        GuildTable::insert(pool, id, channel, role).await
    }
}

#[async_trait]
impl JoinedManager<Postgres> for PostTable {
    async fn upcoming(
        pool: &PgPool,
        user: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<JoinedRow>> {
        let user = user.into();

        sqlx::query_as!(
            JoinedRow,
            r#"
            SELECT
                p.id,
                p.activity,
                p.start_time as "start_time: jiff_sqlx::Timestamp",

                COALESCE(
                    (SELECT array_agg(f.user_id) FROM lfg_fireteam f WHERE f.post_id = p.id),
                    '{}'
                ) AS "fireteam!"

            FROM
                lfg_posts p
            JOIN lfg_fireteam f ON p.id = f.post_id
            WHERE
                f.user_id = $1
            "#,
            as_i64(user.get())
        )
        .fetch_all(pool)
        .await
    }
}

#[async_trait]
impl EditManager<Postgres> for PostTable {
    async fn edit_row(pool: &PgPool, id: MessageId) -> sqlx::Result<EditRow> {
        let id = as_i64(id.get());

        sqlx::query_as!(
            EditRow,
            r#"
            SELECT
                p.owner_id AS "owner_id!",
                p.activity AS "activity!",
                p.start_time AS "start_time!: jiff_sqlx::Timestamp",
                p.description AS "description!",
                p.fireteam_size AS "fireteam_size!",
                u.timezone AS "timezone?"
            FROM
                lfg_posts AS p
            LEFT JOIN
                lfg_user_settings AS u ON p.owner_id = u.id
            WHERE
                p.id = $1
            "#,
            id
        )
        .fetch_one(pool)
        .await
    }
}

// endregion

// region: UsersTable

pub struct UsersTable;

#[async_trait]
impl TimezoneManager<Postgres> for UsersTable {
    async fn get(pool: &PgPool, id: UserId, locale: &str) -> sqlx::Result<TimeZone> {
        let row = sqlx::query!(
            "SELECT timezone FROM lfg_user_settings WHERE id = $1",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await?;

        let tz_name = match row {
            Some(row) => row.timezone,
            None => locale_to_timezone(locale).to_string(),
        };

        Ok(tz::db().get(&tz_name).unwrap_or(TimeZone::UTC))
    }

    async fn save(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
        tz_name: &str,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query!(
            "INSERT INTO lfg_user_settings (id, timezone) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET timezone = $2",
            as_i64(id.get()),
        tz_name)
        .execute(pool)
        .await
    }
}

// endregion

// region: Components

pub struct LfgJoin;

#[async_trait]
impl ModuleComponent for LfgJoin {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_join"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::join::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgLeave;

#[async_trait]
impl ModuleComponent for LfgLeave {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_leave"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::leave::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgAlternative;

#[async_trait]
impl ModuleComponent for LfgAlternative {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_alternative"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::alternative::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgSettings;

#[async_trait]
impl ModuleComponent for LfgSettings {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_settings"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::settings::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgEditComponent;

#[async_trait]
impl ModuleComponent for LfgEditComponent {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_edit"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::edit::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgCopy;

#[async_trait]
impl ModuleComponent for LfgCopy {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_copy"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::copy::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgKick;

#[async_trait]
impl ModuleComponent for LfgKick {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_kick"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::kick::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgKickMenu;

#[async_trait]
impl ModuleComponent for LfgKickMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_kick_menu"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        KickComponent::run::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgDelete;

#[async_trait]
impl ModuleComponent for LfgDelete {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_delete"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::delete::<Postgres, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgTagsAdd;

#[async_trait]
impl ModuleComponent for LfgTagsAdd {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_tags_add"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TagsComponent::add(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

pub struct LfgTagsRemove;

#[async_trait]
impl ModuleComponent for LfgTagsRemove {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_tags_remove"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TagsComponent::remove(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

// endregion

// region: Modals

pub struct LfgEditModal;

#[async_trait]
impl ModuleModal for LfgEditModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_edit"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        lfg::modals::Edit::run::<BotState, Postgres, PostTable, UsersTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct LfgCreateModal;

#[async_trait]
impl ModuleModal for LfgCreateModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("lfg_create"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        lfg::modals::Create::run::<
            BotState,
            Postgres,
            GuildTable,
            PostTable,
            UsersTable,
        >(cx.ctx, cx.interaction, &cx.app.db)
        .await?;
        Ok(())
    }
}

// endregion

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Lfg)
        .add_autocomplete(Lfg)
        .add_component(LfgJoin)?
        .add_component(LfgLeave)?
        .add_component(LfgAlternative)?
        .add_component(LfgSettings)?
        .add_component(LfgEditComponent)?
        .add_component(LfgCopy)?
        .add_component(LfgKick)?
        .add_component(LfgKickMenu)?
        .add_component(LfgDelete)?
        .add_component(LfgTagsAdd)?
        .add_component(LfgTagsRemove)?
        .add_modal(LfgEditModal)?
        .add_modal(LfgCreateModal)?;

    Ok(())
}
