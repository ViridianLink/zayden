use serde_json::Value;

use super::scalar;
use crate::model::{Contract, Faction, Upgrade};
use crate::parse::lexical::slugify;

#[must_use]
pub fn cyberacme_faction_to_model(slug: &str, envelope: &Value) -> Faction {
    let faction = envelope.get("faction").unwrap_or(&Value::Null);
    let name = faction
        .get("name")
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    let priority_contracts = envelope
        .get("contracts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(contract)
        .collect();

    let upgrades = envelope
        .get("upgradeTrees")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(upgrade_tree)
        .collect();

    Faction { slug: slug.to_string(), name, priority_contracts, upgrades }
}

fn contract(value: &Value) -> Contract {
    let name = value.get("name").and_then(Value::as_str).unwrap_or_default();
    let slug = value
        .get("slug")
        .and_then(Value::as_str)
        .map_or_else(|| slugify(name), str::to_string);

    let difficulty = value.get("contractType").and_then(scalar).or_else(|| {
        value.get("requiredLevel").and_then(scalar).map(|lvl| format!("Rep {lvl}"))
    });

    Contract {
        slug,
        name: name.to_string(),
        description: value.get("description").and_then(scalar),
        difficulty,
    }
}

fn upgrade_tree(tree: &Value) -> Vec<Upgrade> {
    let nodes =
        tree.get("upgrades").or_else(|| tree.get("nodes")).and_then(Value::as_array);

    nodes
        .into_iter()
        .flatten()
        .filter_map(|node| {
            let name = node.get("name").and_then(Value::as_str)?;
            Some(Upgrade {
                name: name.to_string(),
                cost: node
                    .get("cost")
                    .or_else(|| node.get("creditCost"))
                    .and_then(scalar),
                requirements: node
                    .get("requiredLevel")
                    .and_then(scalar)
                    .map(|lvl| format!("Rep {lvl}")),
            })
        })
        .collect()
}
