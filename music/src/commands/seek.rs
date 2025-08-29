use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Seek;

impl Seek {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        todo!()
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("seek").description("Seek to a position in the current track")
    }
}
