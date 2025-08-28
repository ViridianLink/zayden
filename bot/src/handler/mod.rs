use std::sync::atomic::AtomicBool;

use serenity::all::{EventHandler, FullEvent};
use serenity::async_trait;
use serenity::model::prelude::Interaction;
use serenity::prelude::Context;
use tokio::sync::RwLock;

use crate::ctx_data::CtxData;
use crate::sqlx_lib::PostgresPool;

mod guild_create;
mod interaction;
mod message_create;
mod reaction_add;
mod reaction_remove;
mod ready;
mod thread_delete;
mod voice_state_update;

pub struct Handler {
    pub started_cron: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn dispatch(&self, ctx: &Context, ev: &FullEvent) {
        let event_name: &'static str = ev.into();

        let ev_command_name = match ev {
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(interaction),
                ..
            } => interaction.data.name.as_str(),
            _ => "",
        };

        let pool = {
            let data = ctx.data::<RwLock<CtxData>>();
            let data = data.read().await;
            data.pool().clone()
        };

        let result = match ev {
            FullEvent::GuildCreate { guild, .. } => Self::guild_create(ctx, guild, &pool).await,
            FullEvent::Message { new_message, .. } => {
                Self::message_create(ctx, new_message, &pool).await
            }
            FullEvent::ReactionAdd { add_reaction, .. } => {
                Self::reaction_add(ctx, add_reaction, &pool).await
            }
            FullEvent::ReactionRemove {
                removed_reaction, ..
            } => Self::reaction_remove(ctx, removed_reaction, &pool).await,
            FullEvent::Ready { data_about_bot, .. } => {
                Self::ready(self, ctx, data_about_bot, &pool).await
            }
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
            let msg = if ev_command_name.is_empty() {
                format!("Error handling {event_name}: {e:?}")
            } else {
                format!("Error handling {event_name} | {ev_command_name}: {e:?}")
            };

            eprintln!("\n{msg}\n{ev:?}\n");
        }
    }
}
