use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct PlayNow;

impl PlayNow {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        todo!()
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("playnow")
            .description("Skips the current song and plays the song you requested")
    }
}
