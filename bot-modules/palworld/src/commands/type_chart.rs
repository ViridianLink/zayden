use std::collections::HashMap;

use serenity::all::ResolvedValue;
use zayden_core::{InvocationCtx, required_option};

use super::respond;
use crate::client::PalworldClient;
use crate::error::{PalworldError, Result};
use crate::model::Element;
use crate::{embeds, typechart};

const MAX_PALS: usize = 40;

pub(super) async fn run(
    cx: &InvocationCtx<'_>,
    client: &PalworldClient,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let raw: &str =
        required_option(&mut options, "element").map_err(PalworldError::from)?;
    let element = Element::parse(raw)
        .ok_or_else(|| PalworldError::UnknownElement(raw.to_string()))?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let pals = client.pals().await?;
    let mut names: Vec<String> = pals
        .iter()
        .filter(|p| p.elements.contains(&element))
        .map(|p| p.name.clone())
        .collect();
    names.sort();
    names.truncate(MAX_PALS);

    let strong = typechart::strong_against(element);
    let weak = typechart::weak_to(element);

    respond(cx, embeds::type_component(element, strong, &weak, &names)).await
}
