use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Mentionable,
};
use sqlx::PgPool;

use crate::{LLAMA_USER, Result};

const COUNTER: &str = "dumb_count";

pub struct Goof;

impl Goof {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let dumb_count = sqlx::query_scalar!(
            "INSERT INTO llamad2_counters (name, count)
                 VALUES ($1, 1)
             ON CONFLICT (name)
                 DO UPDATE SET count = llamad2_counters.count + 1
             RETURNING count",
            COUNTER,
        )
        .fetch_one(pool)
        .await?;

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
