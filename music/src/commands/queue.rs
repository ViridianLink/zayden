use serenity::all::{CommandInteraction, Context, CreateCommand, EditInteractionResponse};
use tokio::sync::RwLock;

use crate::MusicData;

pub struct Queue;

impl Queue {
    pub async fn run<Data: MusicData>(ctx: &Context, interaction: &CommandInteraction) {
        todo!()
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        CreateCommand::new("queue").description("View the current music queue")
    }
}
