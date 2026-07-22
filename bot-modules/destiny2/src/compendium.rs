use google_sheets_api::SheetsClientBuilder;
use sqlx::PgPool;
use zayden_core::CoreError;

use crate::Result;
use crate::db::compendium;

const COMPENDIUM_ID: &str = "1WaxvbLx7UoSZaBqdFr1u32F2uWVLo-CJunJB4nlGUE4";

#[derive(Debug, PartialEq, Eq)]
pub struct PerkInfo {
    pub name: String,
    pub description: String,
}

pub async fn update(pool: &PgPool, api_key: &str) -> Result<()> {
    let client = SheetsClientBuilder::new(api_key).build()?;

    let spreadsheet = client.spreadsheet(COMPENDIUM_ID, true).await?;

    let mut perks_sheet = spreadsheet
        .sheets
        .into_iter()
        .find(|sheet| sheet.properties.title.eq_ignore_ascii_case("gear perks"))
        .ok_or_else(|| CoreError::missing_data("gear perks sheet"))?;

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

    compendium::replace(pool, &perks).await?;

    Ok(())
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
