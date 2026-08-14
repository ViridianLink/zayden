use std::borrow::Cow;

use async_trait::async_trait;
use greetings::{register as register_command, run};
use serenity::all::CreateCommand;
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(GoodCmd);
}

pub struct GoodCmd;

#[async_trait]
impl ModuleCommand for GoodCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("good")
    }

    fn module(&self) -> Option<&'static str> {
        Some("greetings")
    }

    fn definition(&self) -> CreateCommand<'static> {
        register_command()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        run(cx, &cx.app.settings.greetings).await?;
        Ok(())
    }
}
