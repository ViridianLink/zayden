use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Replay;

impl Replay {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        todo!()
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("replay").description("Resets the progress of the current song")
    }
}
