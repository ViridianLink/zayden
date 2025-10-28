use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::LLAMA_GUILD;

const CONTENT: &str = "This is Llama's Raid Report: <https://raid.report/ps/4611686018441992331>";

pub struct RaidReport;

impl RaidReport {
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
        CreateCommand::new("raidreport").description("Returns Llama's Raid Report.")
    }
}
