mod cleanup;
mod clear;
mod control;
mod ctx;
mod disconnect;
mod forceskip;
mod forward;
mod join;
mod r#loop;
mod move_song;
mod nowplaying;
mod pause;
mod play;
mod playnow;
mod playtop;
mod queue;
mod remove;
mod removedupes;
mod replay;
mod resume;
mod rewind;
mod seek;
mod settings;
mod shuffle;
mod skip;
mod skipto;
mod volume;

pub use ctx::{MusicCtx, MusicServices};
use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    ResolvedOption,
};
use zayden_core::parse_subcommand;

use crate::error::{MusicError, Result};

pub struct Command;

impl Command {
    pub fn register() -> CreateCommand<'static> {
        let play = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "play",
            "Play a song or playlist from YouTube or Spotify (queues if something is already playing)",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "A search term, YouTube link, or Spotify link",
            )
            .required(true),
        );

        let join = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "join",
            "Join your current voice channel",
        );

        let disconnect = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "disconnect",
            "Leave the voice channel and clear the queue",
        );

        let nowplaying = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "nowplaying",
            "Show the currently playing track",
        );

        let queue = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "queue",
            "View the current queue",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "page",
            "The page of the queue to view",
        ));

        let pause = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "pause",
            "Pause the current track",
        );

        let resume = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "resume",
            "Resume the current track",
        );

        let replay = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "replay",
            "Restart the current track from the beginning",
        );

        let seek = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "seek",
            "Seek to a position in the current track",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "timestamp",
                "Position to seek to, e.g. 1:23 or 83",
            )
            .required(true),
        );

        let forward = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "forward",
            "Seek forward by a number of seconds",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "secs",
                "Seconds to seek forward",
            )
            .required(true),
        );

        let rewind = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "rewind",
            "Seek backward by a number of seconds",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "secs",
                "Seconds to seek backward",
            )
            .required(true),
        );

        let volume = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "volume",
            "Get or set the playback volume",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "volume",
            "Volume percentage (0-100)",
        ));

        let skip = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "skip",
            "Vote to skip the current track",
        );

        let forceskip = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "forceskip",
            "Immediately skip the current track, bypassing voting",
        );

        let skipto = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "skipto",
            "Skip directly to a position in the queue",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "position",
                "The queue position to jump to",
            )
            .required(true),
        );

        let playnow = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "playnow",
            "Play a track immediately, skipping the current one",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "A search term, YouTube link, or Spotify link",
            )
            .required(true),
        );

        let playtop = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "playtop",
            "Queue a track at the top of the queue",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "A search term, YouTube link, or Spotify link",
            )
            .required(true),
        );

        let remove = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "remove",
            "Remove a track from the queue",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "position",
                "The queue position to remove",
            )
            .required(true),
        );

        let removedupes = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "removedupes",
            "Remove duplicate tracks from the queue",
        );

        let move_song = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "move_song",
            "Move a track to a different position in the queue",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "from",
                "The current position of the track",
            )
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "to",
                "The position to move the track to",
            )
            .required(true),
        );

        let clear = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "clear",
            "Clear the queue, keeping the current track",
        );

        let shuffle = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "shuffle",
            "Shuffle the queue",
        );

        let loop_cmd = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "loop",
            "Set the loop mode",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "mode",
                "The loop mode",
            )
            .required(true)
            .add_string_choice("Off", "off")
            .add_string_choice("Track", "track")
            .add_string_choice("Queue", "queue"),
        );

        let cleanup = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "cleanup",
            "Remove queued tracks whose requester has left voice",
        );

        let control = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "control",
            "Post an interactive control panel for the current track",
        );

        let settings = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "settings",
            "View or change this server's music settings",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Role,
            "dj_role",
            "Role required to use privileged music commands",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "clear_dj_role",
            "Clear the DJ role, making privileged commands open to everyone",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "default_volume",
            "Default playback volume percentage (0-100)",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "auto_disconnect_secs",
            "Seconds of inactivity before auto-disconnecting",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "announce_now_playing",
            "Whether to announce each new track",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "stay_connected",
            "24/7 mode: never auto-disconnect (premium)",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "autoplay",
            "Continue playing similar tracks when the queue empties (premium)",
        ));

        CreateCommand::new("music")
            .description("Play music in a voice channel")
            .add_option(play)
            .add_option(join)
            .add_option(disconnect)
            .add_option(nowplaying)
            .add_option(queue)
            .add_option(pause)
            .add_option(resume)
            .add_option(replay)
            .add_option(seek)
            .add_option(forward)
            .add_option(rewind)
            .add_option(volume)
            .add_option(skip)
            .add_option(forceskip)
            .add_option(skipto)
            .add_option(playnow)
            .add_option(playtop)
            .add_option(remove)
            .add_option(removedupes)
            .add_option(move_song)
            .add_option(clear)
            .add_option(shuffle)
            .add_option(loop_cmd)
            .add_option(cleanup)
            .add_option(control)
            .add_option(settings)
    }

    pub async fn run(
        ctx: &MusicCtx<'_>,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        let (name, sub_options) =
            parse_subcommand(options).map_err(MusicError::from)?;
        let options = zayden_core::parse_options(sub_options);

        match name {
            "play" => play::run(ctx, options).await,
            "join" => join::run(ctx).await,
            "disconnect" => disconnect::run(ctx).await,
            "nowplaying" => nowplaying::run(ctx).await,
            "queue" => queue::run(ctx, options).await,
            "pause" => pause::run(ctx).await,
            "resume" => resume::run(ctx).await,
            "replay" => replay::run(ctx).await,
            "seek" => seek::run(ctx, options).await,
            "forward" => forward::run(ctx, options).await,
            "rewind" => rewind::run(ctx, options).await,
            "volume" => volume::run(ctx, options).await,
            "skip" => skip::run(ctx).await,
            "forceskip" => forceskip::run(ctx).await,
            "skipto" => skipto::run(ctx, options).await,
            "playnow" => playnow::run(ctx, options).await,
            "playtop" => playtop::run(ctx, options).await,
            "remove" => remove::run(ctx, options).await,
            "removedupes" => removedupes::run(ctx).await,
            "move_song" => move_song::run(ctx, options).await,
            "clear" => clear::run(ctx).await,
            "shuffle" => shuffle::run(ctx).await,
            "loop" => r#loop::run(ctx, options).await,
            "cleanup" => cleanup::run(ctx).await,
            "control" => control::run(ctx).await,
            "settings" => settings::run(ctx, options).await,
            _ => Err(MusicError::Internal(format!("unexpected subcommand: {name}"))),
        }
    }
}
