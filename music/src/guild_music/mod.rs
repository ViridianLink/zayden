pub mod music_queue;

pub use music_queue::MusicQueue;

#[derive(Default)]
pub struct GuildMusic {
    queue: MusicQueue,
}

impl GuildMusic {
    pub fn queue(&self) -> &MusicQueue {
        &self.queue
    }

    pub fn queue_mut(&mut self) -> &mut MusicQueue {
        &mut self.queue
    }
}
