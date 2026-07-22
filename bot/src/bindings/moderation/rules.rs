use std::path::PathBuf;

use async_trait::async_trait;
use serenity::all::{
    Colour,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    EditMessage,
    GenericChannelId,
    MessageId,
    Permissions,
};
use zayden_core::error::CoreError;
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

const CHANNEL_ID: GenericChannelId = GenericChannelId::new(747_430_712_617_074_718);
const MESSAGE_ID: MessageId = MessageId::new(788_539_168_980_336_701);

pub(super) struct RulesCommand;

#[async_trait]
impl ModuleCommand for RulesCommand {
    fn module(&self) -> Option<&'static str> {
        Some("moderation")
    }

    fn definition(&self) -> CreateCommand<'static> {
        CreateCommand::new("rules")
            .description("Display the server rules")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

        let rules_path = PathBuf::from("messages").join("rules.md");

        let rules = tokio::fs::read_to_string(rules_path)
            .await
            .map_err(|e| CoreError::Other(e.to_string()))?;
        let fields =
            rules.split("\r\n\r\n").filter(|r| !r.trim().is_empty()).map(|rule| {
                let mut lines = rule.lines();
                let title = lines.next().unwrap_or_default().to_string();
                let description = lines.collect::<Vec<&str>>().join("\n");
                (title, description, false)
            });

        let embed = CreateEmbed::new()
            .colour(Colour::from_rgb(255, 0, 0))
            .title("College Kings Server Rules")
            .description("The below rules are a truncated version of the rules found in the [Code of Conduct](https://gist.github.com/KiloOscarSix/201a919b5650e511f11287c0a9c4c2fc).")
            .fields(fields);

        let mut message = cx.ctx.http.get_message(CHANNEL_ID, MESSAGE_ID).await?;
        message.edit(&cx.ctx.http, EditMessage::new().embed(embed)).await?;

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new().content("The rules have been sent."),
            )
            .await?;

        Ok(())
    }
}
