use std::borrow::Cow;

use async_trait::async_trait;
use music::autocomplete;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

pub struct MusicAutocomplete;

#[async_trait]
impl ModuleAutocomplete for MusicAutocomplete {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("music")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        autocomplete::run(cx).await?;
        Ok(())
    }
}
