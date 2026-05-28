use std::sync::Arc;

use serenity::all::{Event, EventHandler, FullEvent, RatelimitInfo};
use serenity::async_trait;
use serenity::model::prelude::Interaction;
use serenity::prelude::Context;
use tokio::sync::RwLock;
use tracing::{error, trace, warn};

mod guild_create;
mod interaction;
mod message_create;
mod presence_update;
mod reaction_add;
mod reaction_remove;
mod ready;
mod thread_delete;
mod voice_state_update;

use crate::BotState;

pub struct Handler {
    pub bot_state: Arc<RwLock<BotState>>,
}

#[async_trait]
impl EventHandler for Handler {
    fn filter_event(&self, _context: &Context, event: Box<Event>) -> Option<Box<Event>> {
        match &*event {
            Event::TypingStart(_) | Event::MessageUpdate(_) => None,
            _ => Some(event),
        }
    }

    async fn dispatch(&self, ctx: &Context, ev: &FullEvent) {
        let event_name: &'static str = ev.into();

        let ev_command_name = match ev {
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(interaction),
                ..
            } => interaction.data.name.as_str(),
            _ => "",
        };

        let pool = self.bot_state.read().await.app.db.clone();

        let result = match ev {
            FullEvent::GuildCreate { guild, .. } => {
                Self::guild_create(self, ctx, guild, &pool).await
            }
            FullEvent::Message { new_message, .. } => {
                Self::message_create(ctx, new_message, &pool).await
            }
            FullEvent::ReactionAdd { add_reaction, .. } => {
                Self::reaction_add(ctx, add_reaction, &pool).await
            }
            FullEvent::ReactionRemove {
                removed_reaction, ..
            } => Self::reaction_remove(ctx, removed_reaction, &pool).await,
            FullEvent::PresenceUpdate { new_data, .. } => {
                Self::presence_update(self, ctx, new_data).await
            }

            FullEvent::Ready { data_about_bot, .. } => Self::ready(self, ctx, data_about_bot).await,
            FullEvent::VoiceStateUpdate { new, .. } => {
                Self::voice_state_update(ctx, new, &pool).await
            }
            FullEvent::InteractionCreate { interaction, .. } => {
                Self::interaction_create(ctx, interaction, &pool).await
            }
            FullEvent::ThreadDelete { thread, .. } => Self::thread_delete(ctx, thread, &pool).await,

            _ => Ok(()),
        };

        if let Err(e) = result {
            error!(
                error = ?e,
                event = event_name,
                command = ev_command_name,
                "error handling event",
            );
        }
    }

    async fn ratelimit(&self, data: RatelimitInfo) {
        if !data.path.ends_with("commands") {
            warn!(
                path = %data.path,
                method = ?data.method,
                limit = data.limit,
                timeout_ms = data.timeout.as_millis() as u64,
                "rate limited",
            );
        } else {
            trace!(?data, "rate limited (commands)");
        }
    }
}
