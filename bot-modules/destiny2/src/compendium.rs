use std::collections::HashMap;
use std::env;

use google_sheets_api::SheetsClientBuilder;
use serde::{Deserialize, Serialize};

const COMPENDIUM_ID: &str = "1WaxvbLx7UoSZaBqdFr1u32F2uWVLo-CJunJB4nlGUE4";

#[derive(Deserialize, Serialize)]
pub struct PerkInfo {
    pub name: String,
    pub description: String,
}

pub async fn update() {
    let api_key = env::var("GOOGLE_API_KEY").unwrap();

    let client = SheetsClientBuilder::new(api_key).build().unwrap();

    let spreadsheet = client.spreadsheet(COMPENDIUM_ID, true).await.unwrap();
    let mut perks_sheet = spreadsheet
        .sheets
        .into_iter()
        .find(|sheet| sheet.properties.title.eq_ignore_ascii_case("gear perks"))
        .unwrap();
    let data = perks_sheet.data.pop().unwrap();

    let perks = data
        .row_data
        .into_iter()
        .skip(5)
        .filter_map(|row| {
            let mut values = row.values;

            if let (Some(description), Some(name)) = (
                values.swap_remove(2).formatted_value,
                values.swap_remove(0).formatted_value,
            ) {
                let name = name.split("\n\n").next().unwrap().replace("\n", " ");

                Some((name.to_lowercase(), PerkInfo { name, description }))
            } else {
                None
            }
        })
        .collect::<HashMap<String, PerkInfo>>();

    let json = serde_json::to_string(&perks).unwrap();
    std::fs::write("perks.json", json).unwrap();
}
