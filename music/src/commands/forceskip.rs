use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct ForceSkip;

impl ForceSkip {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        interaction.defer(&ctx.http).await.unwrap();

        let guild = interaction.guild_id.unwrap();

        let handle = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            match data.queue(guild) {
                Some(queue) => queue.nowplaying().await.unwrap(),
                None => return,
            }
        };

        handle.stop().unwrap();

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Track skipped"),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("forceskip").description("Force skips current track")
    }
}
