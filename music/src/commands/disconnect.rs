use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Disconnect;

impl Disconnect {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        interaction.defer(&ctx.http).await.unwrap();

        let manager = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            data.songbird()
        };

        let guild = interaction.guild_id.unwrap();
        manager.remove(guild).await.unwrap();

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Disconnected"),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("disconnect").description("Disconnects from voice channel")
    }
}
