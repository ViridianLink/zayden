use std::{
    fs::OpenOptions,
    io::{BufReader, Write},
};

use serde::{Deserialize, Serialize};
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, Mentionable,
};

use crate::{LLAMA_GUILD, LLAMA_USER};

const FILE_NAME: &str = "dumbCount.json";

pub struct Goof;

impl Goof {
    pub async fn run(ctx: &Context, interaction: &CommandInteraction) {
        if interaction
            .guild_id
            .is_none_or(|guild| guild != LLAMA_GUILD)
            || interaction.user.bot()
        {
            return;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .truncate(false)
            .write(true)
            .open(FILE_NAME)
            .unwrap();
        let data = BufReader::new(&file);

        let mut data = serde_json::from_slice::<GoofData>(data.buffer()).unwrap_or_default();
        data.dumb_count += 1;
        file.write_all(serde_json::to_string(&data).unwrap().as_bytes())
            .unwrap();

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
            .await
            .unwrap();
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
