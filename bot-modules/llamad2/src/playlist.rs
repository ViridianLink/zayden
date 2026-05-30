use serenity::Error;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::LLAMA_GUILD;

const CONTENT: &str = "Here is Llama's stream playlist - <https://open.spotify.com/playlist/2WLXsl0kbwKuHlTcrqe2L2?si=674467318f3044c0>";

pub struct Playlist;

impl Playlist {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), Error> {
        if interaction.guild_id.is_none_or(|guild| guild != LLAMA_GUILD)
            || interaction.user.bot()
        {
            return Ok(());
        }

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(CONTENT),
                ),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("playlist")
            .description("Returns Llama's Spotify stream playlist.")
    }
}
