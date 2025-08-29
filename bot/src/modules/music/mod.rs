pub mod clear;
pub mod disconnect;
pub mod forceskip;
pub mod forward;
pub mod join;
pub mod loop_song;
pub mod move_song;
pub mod nowplaying;
pub mod pause;
pub mod play;
pub mod playnow;
pub mod playtop;
pub mod queue;
pub mod remove;
pub mod removedupes;
pub mod replay;
pub mod resume;
pub mod rewind;
pub mod seek;
pub mod shuffle;
pub mod skip;
pub mod skipto;
pub mod volume;

pub use clear::Clear;
pub use disconnect::Disconnect;
pub use forceskip::ForceSkip;
pub use forward::Forward;
pub use join::Join;
pub use loop_song::LoopSong;
pub use move_song::MoveSong;
pub use nowplaying::NowPlaying;
pub use pause::Pause;
pub use play::Play;
pub use playnow::PlayNow;
pub use playtop::PlayTop;
pub use queue::Queue;
pub use remove::Remove;
pub use removedupes::RemoveDupes;
pub use replay::Replay;
pub use resume::Resume;
pub use rewind::Rewind;
pub use seek::Seek;
use serenity::all::{Context, CreateCommand};
pub use shuffle::Shuffle;
pub use skip::Skip;
pub use skipto::SkipTo;
pub use volume::Volume;
use zayden_core::ApplicationCommand;

pub fn register(ctx: &Context) -> [CreateCommand<'_>; 23] {
    [
        Clear::register(ctx).unwrap(),
        Disconnect::register(ctx).unwrap(),
        ForceSkip::register(ctx).unwrap(),
        Forward::register(ctx).unwrap(),
        Join::register(ctx).unwrap(),
        LoopSong::register(ctx).unwrap(),
        MoveSong::register(ctx).unwrap(),
        NowPlaying::register(ctx).unwrap(),
        Pause::register(ctx).unwrap(),
        Play::register(ctx).unwrap(),
        PlayNow::register(ctx).unwrap(),
        PlayTop::register(ctx).unwrap(),
        Queue::register(ctx).unwrap(),
        Remove::register(ctx).unwrap(),
        RemoveDupes::register(ctx).unwrap(),
        Replay::register(ctx).unwrap(),
        Resume::register(ctx).unwrap(),
        Rewind::register(ctx).unwrap(),
        Seek::register(ctx).unwrap(),
        Shuffle::register(ctx).unwrap(),
        Skip::register(ctx).unwrap(),
        SkipTo::register(ctx).unwrap(),
        Volume::register(ctx).unwrap(),
    ]
}
