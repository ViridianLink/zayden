use std::borrow::Cow;

use async_trait::async_trait;
use greetings::{GreetingKind, register as register_command, run};
use serenity::all::CreateCommand;
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(GoodMorningCmd);
    builder.add_command(GoodNightCmd);
}

pub struct GoodMorningCmd;

#[async_trait]
impl ModuleCommand for GoodMorningCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(GreetingKind::Morning.command_name())
    }

    fn module(&self) -> Option<&'static str> {
        Some("greetings")
    }

    fn definition(&self) -> CreateCommand<'static> {
        register_command(GreetingKind::Morning)
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        run(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
            &cx.app.settings.greetings,
            GreetingKind::Morning,
        )
        .await?;
        Ok(())
    }
}

pub struct GoodNightCmd;

#[async_trait]
impl ModuleCommand for GoodNightCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(GreetingKind::Night.command_name())
    }

    fn module(&self) -> Option<&'static str> {
        Some("greetings")
    }

    fn definition(&self) -> CreateCommand<'static> {
        register_command(GreetingKind::Night)
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        run(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
            &cx.app.settings.greetings,
            GreetingKind::Night,
        )
        .await?;
        Ok(())
    }
}
