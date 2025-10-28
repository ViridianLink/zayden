use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::LLAMA_GUILD;

const CONTENT: &str = "Llama is on 7 move and 0.7 ads, with 800 dpi.";

pub struct Sensitivity;

impl Sensitivity {
    pub async fn run(ctx: &Context, interaction: &CommandInteraction) {
        if interaction
            .guild_id
            .is_none_or(|guild| guild != LLAMA_GUILD)
            || interaction.user.bot()
        {
            return;
        }

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(CONTENT),
                ),
            )
            .await
            .unwrap();
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("sensitivity").description("Returns Llama's game sensitivity settings.")
    }
}
