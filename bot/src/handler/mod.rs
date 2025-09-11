use std::sync::atomic::AtomicBool;

use serenity::all::{Event, EventHandler, FullEvent, GuildCreateEvent, RatelimitInfo};
use serenity::async_trait;
use serenity::model::prelude::Interaction;
use serenity::prelude::Context;
use sqlx::PgPool;

use crate::{BRADSTER_GUILD, ZAYDEN_GUILD};

mod guild_create;
mod interaction;
mod message_create;
mod reaction_add;
mod reaction_remove;
mod ready;
mod thread_delete;
mod voice_state_update;

pub struct Handler {
    pub pool: PgPool,
    pub started_cron: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    fn filter_event(&self, _ctx: &Context, event: &Event) -> bool {
        match event {
            Event::GuildCreate(GuildCreateEvent { guild, .. })
                if guild.id == BRADSTER_GUILD || guild.id == ZAYDEN_GUILD =>
            {
                println!("[{}] Registered {}", event.name(), guild.name);
                true
            }
            Event::PresenceUpdate(_) | Event::TypingStart(_) | Event::MessageUpdate(_) => false,
            _ => true,
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

        let result = match ev {
            FullEvent::GuildCreate { guild, .. } => {
                Self::guild_create(ctx, guild, &self.pool).await
            }
            FullEvent::Message { new_message, .. } => {
                Self::message_create(ctx, new_message, &self.pool).await
            }
            FullEvent::ReactionAdd { add_reaction, .. } => {
                Self::reaction_add(ctx, add_reaction, &self.pool).await
            }
            FullEvent::ReactionRemove {
                removed_reaction, ..
            } => Self::reaction_remove(ctx, removed_reaction, &self.pool).await,
            FullEvent::Ready { data_about_bot, .. } => Self::ready(self, ctx, data_about_bot).await,
            FullEvent::VoiceStateUpdate { new, .. } => {
                Self::voice_state_update(ctx, new, &self.pool).await
            }
            FullEvent::InteractionCreate { interaction, .. } => {
                Self::interaction_create(ctx, interaction, &self.pool).await
            }
            FullEvent::ThreadDelete { thread, .. } => {
                Self::thread_delete(ctx, thread, &self.pool).await
            }
            _ => Ok(()),
        };

        if let Err(e) = result {
            let msg = if ev_command_name.is_empty() {
                format!("Error handling {event_name}: {e:?}")
            } else {
                format!("Error handling {event_name} | {ev_command_name}: {e:?}")
            };

            eprintln!("\n{msg}\n{ev:?}\n");
        }
    }

    async fn ratelimit(&self, data: RatelimitInfo) {
        println!("{:?}", data)
    }
}
