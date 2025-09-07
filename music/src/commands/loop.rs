use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use songbird::tracks::LoopState;
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Loop;

impl Loop {
    pub async fn run<Data: MusicData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
    ) {
        interaction.defer(&ctx.http).await.unwrap();

        let Some(ResolvedValue::String(mode)) = options.pop().map(|opt| opt.value) else {
            unreachable!("Option should be required")
        };

        let guild = interaction.guild_id.unwrap();

        let handle = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            match data.queue(guild) {
                Some(queue) => queue.nowplaying().await.unwrap(),
                None => return,
            }
        };

        // TODO: Add queue looping

        if mode == "track" {
            match handle.get_info().await {
                Ok(info) if matches!(info.loops, LoopState::Finite(0)) => {
                    handle.enable_loop().unwrap()
                }
                Ok(_) | Err(_) => handle.disable_loop().unwrap(),
            };
        }

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Looping toggled"),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("loop")
            .description("Loop the current song or entire queue")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "mode",
                    "Loop current track or entire queue",
                )
                .add_string_choice("Track", "track")
                // .add_string_choice("Queue", "queue")
                .required(true),
            )
    }
}
