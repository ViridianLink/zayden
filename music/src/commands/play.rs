use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use songbird::input::YoutubeDl;
use tokio::sync::RwLock;

use crate::MusicData;
use crate::actions::play;

pub struct Play;

impl Play {
    pub async fn run<Data: MusicData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
    ) {
        interaction.defer(&ctx.http).await.unwrap();

        let Some(ResolvedValue::String(track)) = options.pop().map(|opt| opt.value) else {
            unreachable!("Track option is required")
        };

        let (http_client, manager) = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            (data.http(), data.songbird())
        };

        let src = match url::Url::parse(track) {
            Ok(url) => YoutubeDl::new(http_client, url.to_string()),
            Err(_) => YoutubeDl::new_search(http_client, track.to_string()),
        };

        let guild_id = interaction.guild_id.unwrap();

        let title = play::<Data>(ctx, &manager, guild_id, interaction.user.id, src).await;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content(format!("Queued: **{title}**")),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("play")
            .description("Play a track (supports search or links)")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "song", "Search query or link")
                    .required(true),
            )
    }
}
