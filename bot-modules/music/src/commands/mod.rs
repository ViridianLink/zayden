mod clear;
mod control;
mod ctx;
mod disconnect;
mod join;
mod r#loop;
mod move_song;
mod nowplaying;
mod pause;
mod play;
mod playnow;
mod playtop;
mod queue;
mod radio;
mod remove;
mod replay;
mod resume;
mod seek;
mod settings;
mod shuffle;
mod silent;
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
use zayden_app::config::Genre;
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
                "Absolute (1:23, 83) or relative (+30, -1:30) position",
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
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "force",
            "Immediately skip, bypassing voting (requires privileges)",
        ));

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
            "Prune the queue, keeping the current track",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "mode",
                "What to remove (defaults to the whole queue)",
            )
            .add_string_choice("Everything", "all")
            .add_string_choice("Duplicates", "duplicates")
            .add_string_choice("Requester left voice", "left"),
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

        let genre = Genre::ALL.into_iter().fold(
            CreateCommandOption::new(
                CommandOptionType::String,
                "genre",
                "Genre or mood to stream",
            )
            .required(true),
            |option, genre| option.add_string_choice(genre.label(), genre.value()),
        );

        let radio = CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            "radio",
            "Play a curated genre or mood radio stream",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "play",
                "Start streaming a genre or mood",
            )
            .add_sub_option(genre),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "stop",
            "Stop the radio and resume the queue",
        ));

        let control = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "control",
            "Post an interactive control panel for the current track",
        );

        let silent = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "silent",
            "Silence now-playing announcements for this session",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Boolean,
            "enabled",
            "Leave blank to toggle",
        ));

        let settings = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "settings",
            "View or change this server's music settings",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Integer,
            "default_volume",
            "Default playback volume percentage (0-100)",
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
            .add_option(volume)
            .add_option(skip)
            .add_option(skipto)
            .add_option(playnow)
            .add_option(playtop)
            .add_option(remove)
            .add_option(move_song)
            .add_option(clear)
            .add_option(shuffle)
            .add_option(loop_cmd)
            .add_option(radio)
            .add_option(control)
            .add_option(silent)
            .add_option(settings)
    }

    pub async fn run(
        ctx: &MusicCtx<'_>,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        let (name, sub_options) =
            parse_subcommand(options).map_err(MusicError::from)?;

        if name == "radio" {
            return radio::run(ctx, sub_options).await;
        }

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
            "volume" => volume::run(ctx, options).await,
            "skip" => skip::run(ctx, options).await,
            "skipto" => skipto::run(ctx, options).await,
            "playnow" => playnow::run(ctx, options).await,
            "playtop" => playtop::run(ctx, options).await,
            "remove" => remove::run(ctx, options).await,
            "move_song" => move_song::run(ctx, options).await,
            "clear" => clear::run(ctx, options).await,
            "shuffle" => shuffle::run(ctx).await,
            "loop" => r#loop::run(ctx, options).await,
            "control" => control::run(ctx).await,
            "silent" => silent::run(ctx, options).await,
            "settings" => settings::run(ctx, options).await,
            _ => Err(MusicError::Internal(format!("unexpected subcommand: {name}"))),
        }
    }
}
