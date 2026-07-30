use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use honeypot::HoneypotHit;
use serenity::all::{CreateCommand, UserId};
use zayden_app::state::AppState;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::RegistryBuilder;
use crate::bindings::moderation::{InfractionKind, NewInfraction};

const HONEYPOT_POINTS: i32 = 0;

const HONEYPOT_REASON: &str = "Honeypot: posted in the honeypot channel";
const HONEYPOT_MODERATOR: &str = "Zayden (Honeypot)";

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Honeypot);
}

pub struct Honeypot;

#[async_trait]
impl ModuleCommand for Honeypot {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("honeypot")
    }

    fn module(&self) -> Option<&'static str> {
        Some("honeypot")
    }

    fn definition(&self) -> CreateCommand<'static> {
        honeypot::Honeypot::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        honeypot::Honeypot::run(cx).await?;
        Ok(())
    }
}

pub async fn record_hit(app: &Arc<AppState>, hit: &HoneypotHit) -> sqlx::Result<()> {
    NewInfraction {
        guild_id: hit.guild_id,
        target_id: hit.user_id,
        target_username: &hit.username,
        kind: InfractionKind::SoftBan,
        moderator_id: UserId::new(app.zayden_id),
        moderator_username: HONEYPOT_MODERATOR,
        points: HONEYPOT_POINTS,
        reason: HONEYPOT_REASON,
    }
    .record(&app.db)
    .await
}
