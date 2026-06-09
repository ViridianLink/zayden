use std::fs::OpenOptions;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use serenity::Error;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    Mentionable,
};

use crate::LLAMA_USER;

const FILE_NAME: &str = "dumbCount.json";

pub struct Goof;

impl Goof {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(FILE_NAME)?;

        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;

        let mut data = serde_json::from_str::<GoofData>(&buffer)?;

        data.dumb_count += 1;

        let serialized = serde_json::to_string(&data)?;

        file.set_len(0)?;
        file.write_all(serialized.as_bytes())?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(format!(
                        "{} has *now* been dumb {} times! (what a goof)",
                        LLAMA_USER.mention(),
                        data.dumb_count,
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

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoofData {
    dumb_count: u32,
}
