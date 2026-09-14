use std::sync::Arc;

use serenity::all::{GuildThread, Http};
use tracing::{debug, warn};
use zayden_app::state::AppState;

use crate::faq::{FaqContext, on_ticket_opened};
use crate::idle::ThreadActivity;
use crate::opening::ticket_opening;
use crate::{Result, TicketGuildRow, TicketStores};

const OPENING_ATTEMPTS: u32 = 6;

pub struct SupportThreadCreate;

impl SupportThreadCreate {
    pub async fn run(
        http: &Arc<Http>,
        thread: &GuildThread,
        newly_created: Option<bool>,
        app: &Arc<AppState>,
    ) -> Result<()> {
        if newly_created != Some(true) {
            return Ok(());
        }

        let guild_id = thread.base.guild_id;
        let stores = TicketStores::from_app(app);

        let Some(row) = TicketGuildRow::get(stores, &app.db, guild_id).await? else {
            debug!(%guild_id, "no ticket configuration for guild; ignoring thread");
            return Ok(());
        };

        if row.channel_id() != Some(thread.parent_id) {
            debug!(
                %guild_id,
                thread_id = %thread.id,
                "thread is not in the support channel; ignoring",
            );
            return Ok(());
        }

        ThreadActivity::insert(&app.db, guild_id, thread.id, thread.owner_id)
            .await?;

        let context = match FaqContext::load(stores.faq, guild_id).await {
            Ok(Some(context)) => context,
            Ok(None) => return Ok(()),
            Err(e) => {
                warn!(error = ?e, %guild_id, "could not load faq settings");
                return Ok(());
            },
        };

        if !context.auto_triage {
            return Ok(());
        }

        let Some(opening) = ticket_opening(http, thread, OPENING_ATTEMPTS).await
        else {
            warn!(
                thread_id = %thread.id,
                "support thread opened without a readable message or title; skipping triage",
            );
            return Ok(());
        };

        on_ticket_opened(Arc::clone(http), Arc::clone(app), context, opening);

        Ok(())
    }
}
