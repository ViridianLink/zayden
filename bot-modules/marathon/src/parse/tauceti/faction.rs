use serde_json::Value;

use super::field;
use crate::model::{Contract, Faction, Upgrade};

#[must_use]
pub fn tauceti_faction_to_model(slug: &str, value: &Value) -> Faction {
    Faction {
        slug: slug.to_string(),
        name: field(value, "name").unwrap_or_else(|| slug.to_string()),
        priority_contracts: contracts(value),
        upgrades: upgrades(value),
    }
}

fn contracts(value: &Value) -> Vec<Contract> {
    value
        .get("contracts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|c| Contract {
            slug: field(c, "slug").unwrap_or_default(),
            name: field(c, "name").unwrap_or_default(),
            description: field(c, "description"),
            difficulty: field(c, "difficulty")
                .or_else(|| field(c, "tier"))
                .or_else(|| field(c, "contractType")),
        })
        .filter(|c| !c.name.is_empty())
        .collect()
}

fn upgrades(value: &Value) -> Vec<Upgrade> {
    value
        .get("upgrades")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|u| {
            let name = field(u, "name")?;
            Some(Upgrade {
                name,
                cost: field(u, "cost").or_else(|| field(u, "credits")),
                requirements: field(u, "requirements")
                    .or_else(|| field(u, "requirement")),
            })
        })
        .collect()
}
