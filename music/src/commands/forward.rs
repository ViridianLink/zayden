use std::time::Duration;

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Forward;

impl Forward {
    pub async fn run<Data: MusicData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
    ) {
        interaction.defer(&ctx.http).await.unwrap();

        let Some(ResolvedValue::String(time)) = options.pop().map(|opt| opt.value) else {
            unreachable!("time should be required")
        };

        let secs = time.parse::<u64>().unwrap();
        let position = Duration::from_secs(secs);

        let guild = interaction.guild_id.unwrap();

        let handle = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            match data.queue(guild) {
                Some(queue) => queue.nowplaying().await.unwrap(),
                None => return,
            }
        };

        todo!("Add time to current position");

        handle.seek(position).result().unwrap();

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().content(""))
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("forward")
            .description("Forwards by a certain amount of time in the current track")
            .add_option(CreateCommandOption::new(
                CommandOptionType::String,
                "time",
                "Time to seek forward (in seconds)",
            ))
    }
}
