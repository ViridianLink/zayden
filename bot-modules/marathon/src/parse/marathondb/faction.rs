use serde_json::Value;

use crate::model::{Contract, Faction};
use crate::parse::lexical::slugify;

#[must_use]
pub fn marathondb_contracts_to_factions(data: &[Value]) -> Vec<Faction> {
    let mut factions: Vec<Faction> = Vec::new();

    for contract in data {
        let Some(faction_slug) =
            contract.get("faction_slug").and_then(Value::as_str)
        else {
            continue;
        };
        let Some(name) = contract.get("name").and_then(Value::as_str) else {
            continue;
        };
        let faction_name = contract
            .get("faction_name")
            .and_then(Value::as_str)
            .unwrap_or(faction_slug);

        let slug = contract
            .get("slug")
            .and_then(Value::as_str)
            .map_or_else(|| slugify(name), str::to_string);

        let entry = Contract {
            slug,
            name: name.to_string(),
            description: contract
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            difficulty: contract
                .get("difficulty")
                .and_then(Value::as_str)
                .map(str::to_string),
        };

        if let Some(faction) = factions.iter_mut().find(|f| f.slug == faction_slug) {
            faction.priority_contracts.push(entry);
        } else {
            factions.push(Faction {
                slug: faction_slug.to_string(),
                name: faction_name.to_string(),
                priority_contracts: vec![entry],
                upgrades: Vec::new(),
            });
        }
    }

    factions
}
