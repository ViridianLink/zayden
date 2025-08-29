use async_trait::async_trait;
use serenity::all::{Context, GuildId, UserId};
use songbird::tracks::Track;
use songbird::{Event, EventContext, EventHandler, Songbird, TrackEvent};
use tokio::sync::RwLock;

use crate::MusicData;

use super::{connect, deafen};

pub async fn play<Data: MusicData>(
    ctx: &Context,
    manager: &Songbird,
    guild: GuildId,
    user: UserId,
    track: impl Into<Track>,
) -> String {
    let voice_state = guild.get_user_voice_state(&ctx.http, user).await.unwrap();

    let handler_lock = connect(manager, guild, voice_state.channel_id.unwrap())
        .await
        .unwrap();

    let mut handler = handler_lock.lock().await;
    handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    deafen(&mut handler).await.unwrap();

    let data = ctx.data::<RwLock<Data>>();
    let mut data = data.write().await;
    let queue = data.queue_mut(guild);
    queue.add(&mut handler, track).await
}

struct TrackErrorNotifier;

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}
