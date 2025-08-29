use serenity::all::{
    CommandInteraction, Context, CreateCommand, EditInteractionResponse, Mentionable,
};
use tokio::sync::RwLock;

use crate::MusicData;
use crate::actions::connect;

pub struct Join;

impl Join {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        interaction.defer(&ctx.http).await.unwrap();

        let manager = {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            data.songbird()
        };

        let guild_id = interaction.guild_id.unwrap();
        let voice_state = guild_id
            .get_user_voice_state(&ctx.http, interaction.user.id)
            .await
            .unwrap();
        let channel = voice_state.channel_id.unwrap();

        connect(&manager, guild_id, channel).await.unwrap();

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content(format!("Connected to {}", channel.mention())),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("join").description("Joins the voice channel you are in")
    }
}
