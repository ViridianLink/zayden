use std::sync::Arc;

use serenity::all::{Event, EventHandler, FullEvent, RatelimitInfo};
use serenity::async_trait;
use serenity::model::prelude::Interaction;
use serenity::prelude::Context;
use tokio::sync::RwLock;
use tracing::{error, trace, warn};
use zayden_app::state::AppState;

mod entitlement;
mod guild_create;
mod interaction;
mod message_create;
mod presence_update;
mod reaction_add;
mod reaction_remove;
mod ready;
mod thread_delete;
mod voice_state_update;

use crate::{BotState, CommandRegistry};

pub struct Handler {
    pub app: Arc<AppState>,
    pub bot_state: Arc<RwLock<BotState>>,
    pub registry: Arc<CommandRegistry>,
}

#[async_trait]
impl EventHandler for Handler {
    fn filter_event(
        &self,
        _context: &Context,
        event: Box<Event>,
    ) -> Option<Box<Event>> {
        match &*event {
            Event::GuildCreate(_)
            | Event::MessageCreate(_)
            | Event::ReactionAdd(_)
            | Event::ReactionRemove(_)
            | Event::PresenceUpdate(_)
            | Event::Ready(_)
            | Event::VoiceStateUpdate(_)
            | Event::InteractionCreate(_)
            | Event::ThreadDelete(_)
            | Event::EntitlementCreate(_)
            | Event::EntitlementUpdate(_)
            | Event::EntitlementDelete(_) => Some(event),
            Event::CommandPermissionsUpdate(_)
            | Event::AutoModRuleCreate(_)
            | Event::AutoModRuleUpdate(_)
            | Event::AutoModRuleDelete(_)
            | Event::AutoModActionExecution(_)
            | Event::ChannelCreate(_)
            | Event::ChannelDelete(_)
            | Event::ChannelPinsUpdate(_)
            | Event::ChannelUpdate(_)
            | Event::GuildAuditLogEntryCreate(_)
            | Event::GuildBanAdd(_)
            | Event::GuildBanRemove(_)
            | Event::GuildDelete(_)
            | Event::GuildEmojisUpdate(_)
            | Event::GuildIntegrationsUpdate(_)
            | Event::GuildMemberAdd(_)
            | Event::GuildMemberRemove(_)
            | Event::GuildMemberUpdate(_)
            | Event::GuildMembersChunk(_)
            | Event::GuildRoleCreate(_)
            | Event::GuildRoleDelete(_)
            | Event::GuildRoleUpdate(_)
            | Event::GuildStickersUpdate(_)
            | Event::GuildUpdate(_)
            | Event::InviteCreate(_)
            | Event::InviteDelete(_)
            | Event::MessageDelete(_)
            | Event::MessageDeleteBulk(_)
            | Event::MessageUpdate(_)
            | Event::ReactionRemoveAll(_)
            | Event::ReactionRemoveEmoji(_)
            | Event::Resumed(_)
            | Event::SoundboardSounds(_)
            | Event::SoundboardSoundCreate(_)
            | Event::SoundboardSoundUpdate(_)
            | Event::SoundboardSoundsUpdate(_)
            | Event::SoundboardSoundDelete(_)
            | Event::TypingStart(_)
            | Event::UserUpdate(_)
            | Event::VoiceServerUpdate(_)
            | Event::VoiceChannelStatusUpdate(_)
            | Event::WebhookUpdate(_)
            | Event::IntegrationCreate(_)
            | Event::IntegrationUpdate(_)
            | Event::IntegrationDelete(_)
            | Event::StageInstanceCreate(_)
            | Event::StageInstanceUpdate(_)
            | Event::StageInstanceDelete(_)
            | Event::ThreadCreate(_)
            | Event::ThreadUpdate(_)
            | Event::ThreadListSync(_)
            | Event::ThreadMemberUpdate(_)
            | Event::ThreadMembersUpdate(_)
            | Event::GuildScheduledEventCreate(_)
            | Event::GuildScheduledEventUpdate(_)
            | Event::GuildScheduledEventDelete(_)
            | Event::GuildScheduledEventUserAdd(_)
            | Event::GuildScheduledEventUserRemove(_)
            | Event::MessagePollVoteAdd(_)
            | Event::MessagePollVoteRemove(_)
            | Event::ShardStageUpdate(_) => None,
            _ => {
                error!("unhandled event type");
                None
            },
        }
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "future variants are caught in the filter function"
    )]
    async fn dispatch(&self, ctx: &Context, ev: &FullEvent) {
        let event_name: &'static str = ev.into();

        let ev_command_name = match ev {
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(interaction),
                ..
            } => interaction.data.name.as_str(),
            _ => "",
        };

        let pool = self.app.db.clone();

        let result = match ev {
            FullEvent::GuildCreate { guild, .. } => {
                Self::guild_create(self, ctx, guild, &pool).await
            },
            FullEvent::Message { new_message, .. } => {
                let app = Arc::clone(&self.app);
                Self::message_create(ctx, new_message, &pool, app).await
            },
            FullEvent::ReactionAdd { add_reaction, .. } => {
                Self::reaction_add(ctx, add_reaction, &pool).await
            },
            FullEvent::ReactionRemove { removed_reaction, .. } => {
                Self::reaction_remove(ctx, removed_reaction, &pool).await
            },
            FullEvent::PresenceUpdate { new_data, .. } => {
                Self::presence_update(ctx, new_data);
                Ok(())
            },

            FullEvent::Ready { data_about_bot, .. } => {
                Self::ready(self, ctx, data_about_bot).await
            },
            FullEvent::VoiceStateUpdate { new, .. } => {
                Self::voice_state_update(ctx, new, &pool).await
            },
            FullEvent::InteractionCreate { interaction, .. } => {
                let app = Arc::clone(&self.app);
                let registry = Arc::clone(&self.registry);
                Self::interaction_create(ctx, interaction, app, registry).await
            },
            FullEvent::ThreadDelete { thread, .. } => {
                Self::thread_delete(ctx, thread, &pool).await
            },

            FullEvent::EntitlementCreate { entitlement, .. } => {
                let bot_state = self.bot_state.read().await;
                entitlement::entitlement_create(ctx, entitlement, &bot_state).await
            },
            FullEvent::EntitlementUpdate { entitlement, .. } => {
                let bot_state = self.bot_state.read().await;
                entitlement::entitlement_update(ctx, entitlement, &bot_state).await
            },
            FullEvent::EntitlementDelete { entitlement, .. } => {
                let bot_state = self.bot_state.read().await;
                entitlement::entitlement_delete(ctx, entitlement, &bot_state).await
            },

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
        Self::log_ratelimit(&data);
    }
}

impl Handler {
    fn log_ratelimit(data: &RatelimitInfo) {
        if data.path.ends_with("commands") {
            trace!(?data, "rate limited (commands)");
        } else {
            let timeout_ms =
                u64::try_from(data.timeout.as_millis()).unwrap_or(u64::MAX);
            warn!(
                path = %data.path,
                method = ?data.method,
                limit = data.limit,
                timeout_ms,
                "rate limited",
            );
        }
    }
}
