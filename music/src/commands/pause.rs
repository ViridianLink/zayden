use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Pause;

impl Pause {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        todo!()
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("pause").description("Pauses the current song")
    }
}
