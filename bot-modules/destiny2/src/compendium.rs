use google_sheets_api::SheetsClientBuilder;
use sqlx::PgPool;
use tracing::error;
use zayden_core::CoreError;

use crate::Result;
use crate::db::compendium;

const COMPENDIUM_ID: &str = "1WaxvbLx7UoSZaBqdFr1u32F2uWVLo-CJunJB4nlGUE4";

const PERK_SHEET_TITLES: [&str; 2] = ["weapon perks", "gear perks"];

#[derive(Debug, PartialEq, Eq)]
pub struct PerkInfo {
    pub name: String,
    pub description: String,
}

pub async fn update(pool: &PgPool, api_key: &str) -> Result<()> {
    let client = SheetsClientBuilder::new(api_key).build()?;

    let spreadsheet = client.spreadsheet(COMPENDIUM_ID, true).await?;

    let mut sheets = spreadsheet.sheets;

    let Some(index) = PERK_SHEET_TITLES.iter().find_map(|wanted| {
        sheets
            .iter()
            .position(|s| s.properties.title.trim().eq_ignore_ascii_case(wanted))
    }) else {
        let available = sheets
            .iter()
            .map(|s| s.properties.title.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        error!(
            "No perk tab in the compendium (looked for {PERK_SHEET_TITLES:?}); \
             available tabs: [{available}]"
        );
        return Err(CoreError::missing_data("gear perks sheet").into());
    };

    let mut perks_sheet = sheets.swap_remove(index);

    let data = perks_sheet
        .data
        .pop()
        .ok_or_else(|| CoreError::missing_data("gear perks sheet data"))?;

    let perks = data
        .row_data
        .into_iter()
        .skip(5)
        .filter_map(|row| {
            perk_entry(
                row.values.into_iter().map(|cell| cell.formatted_value).collect(),
            )
        })
        .collect::<Vec<(String, PerkInfo)>>();

    let existing = usize::try_from(compendium::count(pool).await?).unwrap_or(0);
    if !is_safe_replace(existing, perks.len()) {
        error!(
            "Refusing compendium refresh: parsed {} perks vs {existing} existing \
             — keeping current catalog (likely upstream sheet or parser drift)",
            perks.len()
        );
        return Ok(());
    }

    compendium::replace(pool, &perks).await?;

    Ok(())
}

#[must_use]
pub const fn is_safe_replace(existing: usize, incoming: usize) -> bool {
    if existing == 0 {
        return true;
    }

    incoming * 2 >= existing
}

#[must_use]
pub fn perk_entry(mut values: Vec<Option<String>>) -> Option<(String, PerkInfo)> {
    if values.len() < 3 {
        return None;
    }

    let description = values.swap_remove(2)?;
    let name = values.swap_remove(0)?;

    let name = name.split("\n\n").next().unwrap_or(&name).replace('\n', " ");

    Some((name.to_lowercase(), PerkInfo { name, description }))
}
