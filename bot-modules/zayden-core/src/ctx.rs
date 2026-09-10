use std::borrow::Cow;
use std::sync::Arc;

use serenity::all::{
    CommandInteraction,
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ModalInteraction,
};
use zayden_app::state::AppState;

pub struct InvocationCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a CommandInteraction,
    pub app: Arc<AppState>,
}

pub struct ComponentCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a ComponentInteraction,
    pub app: Arc<AppState>,
}

impl ComponentCtx<'_> {
    pub async fn ephemeral(
        &self,
        content: impl Into<Cow<'_, str>>,
    ) -> serenity::Result<()> {
        self.interaction
            .create_response(
                &self.ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(content.into())
                        .ephemeral(true),
                ),
            )
            .await
    }
}

pub struct ModalCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a ModalInteraction,
    pub app: Arc<AppState>,
}

pub struct AutocompleteCtx<'a> {
    pub ctx: &'a Context,
    pub interaction: &'a CommandInteraction,
    pub app: Arc<AppState>,
}
