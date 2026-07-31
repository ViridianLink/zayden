use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Mentionable,
};
use sqlx::PgPool;

use crate::{Counter, LLAMA_USER, Result};

pub struct Goof;

impl Goof {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let dumb_count = Counter::bump(pool, Counter::DUMB_COUNT).await?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(format!(
                        "{} has *now* been dumb {} times! (what a goof)",
                        LLAMA_USER.mention(),
                        dumb_count,
                    )),
                ),
            )
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("goof").description("Tell Llama that he's dumb!")
    }
}
