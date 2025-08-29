use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Clear;

impl Clear {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        interaction.defer(&ctx.http).await.unwrap();

        let guild = interaction.guild_id.unwrap();

        {
            let data = ctx.data::<RwLock<Data>>();
            let mut data = data.write().await;
            data.queue_mut(guild).clear().await;
        };

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Cleared Queue"),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("clear").description("Clears the music queue")
    }
}
