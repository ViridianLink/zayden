use std::sync::Arc;
use std::{collections::VecDeque, time::Duration};

use async_trait::async_trait;
use songbird::tracks::TrackHandle;
use songbird::{
    Driver, Event, EventContext, EventHandler, TrackEvent, input::Input, tracks::Track,
};
use tokio::sync::Mutex;

type MusicQueueInner = Arc<Mutex<VecDeque<TrackHandle>>>;

#[derive(Default)]
pub struct MusicQueue(MusicQueueInner);

impl MusicQueue {
    pub async fn add(&mut self, driver: &mut Driver, track: impl Into<Track>) -> String {
        let mut track: Track = track.into();

        let preload = get_preload_time(&mut track.input).await;
        let title = track
            .input
            .aux_metadata()
            .await
            .unwrap()
            .title
            .unwrap_or_default();

        let handle = driver.play(track);

        let mut inner = self.0.lock().await;

        let should_play = inner.is_empty();

        if should_play {
            handle.play().unwrap();
        }

        handle
            .add_event(
                Event::Track(TrackEvent::End),
                QueueHandler {
                    queue_inner: Arc::clone(&self.0),
                },
            )
            .unwrap();

        if let Some(time) = preload {
            handle
                .add_event(
                    Event::Delayed(time),
                    SongPreloader {
                        inner: Arc::clone(&self.0),
                    },
                )
                .unwrap();
        }

        inner.push_back(handle);

        title
    }

    pub async fn clear(&mut self) {
        let mut inner = self.0.lock().await;
        inner.clear();
    }

    pub async fn nowplaying(&self) -> Option<TrackHandle> {
        let inner = self.0.lock().await;
        inner.front().cloned()
    }
}

async fn get_preload_time(input: &mut Input) -> Option<Duration> {
    let meta = match input {
        Input::Lazy(rec) | Input::Live(_, Some(rec)) => rec.aux_metadata().await.ok(),
        Input::Live(_, None) => None,
    };

    meta.and_then(|meta| meta.duration)
        .map(|d| d.saturating_sub(Duration::from_secs(5)))
}

struct QueueHandler {
    queue_inner: MusicQueueInner,
}

#[async_trait]
impl EventHandler for QueueHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let mut inner = self.queue_inner.lock().await;

        match ctx {
            EventContext::Track(ts) => {
                if inner.front()?.uuid() != ts.first()?.1.uuid() {
                    return None;
                }
            }
            _ => return None,
        }

        let _old = inner.pop_front();

        while let Some(new) = inner.front() {
            if new.play().is_err() {
                inner.pop_front();
            } else {
                break;
            }
        }

        None
    }
}

struct SongPreloader {
    inner: MusicQueueInner,
}

#[async_trait]
impl EventHandler for SongPreloader {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let inner = self.inner.lock().await;

        if let Some(track) = inner.get(1) {
            drop(track.make_playable());
        }

        None
    }
}
