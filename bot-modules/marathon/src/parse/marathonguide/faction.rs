use scraper::{ElementRef, Html, Selector};

use super::select_text;
use crate::model::{Contract, Faction, Upgrade};
use crate::parse::html;
use crate::parse::lexical::{non_empty, slugify};

#[must_use]
pub fn marathonguide_html_to_faction(
    slug: &str,
    contracts_page: Option<&str>,
    upgrades_page: Option<&str>,
) -> Faction {
    let contracts_doc = contracts_page.map(html::document);
    let upgrades_doc = upgrades_page.map(html::document);

    let name = contracts_doc
        .as_ref()
        .or(upgrades_doc.as_ref())
        .and_then(faction_name)
        .unwrap_or_else(|| slug.to_string());

    Faction {
        slug: slug.to_string(),
        name,
        priority_contracts: contracts_doc
            .as_ref()
            .map(contracts)
            .unwrap_or_default(),
        upgrades: upgrades_doc.as_ref().map(upgrades).unwrap_or_default(),
    }
}

fn faction_name(doc: &Html) -> Option<String> {
    html::text_of(doc, "title")
        .ok()
        .flatten()
        .and_then(|title| non_empty(title.split(" - ").next().unwrap_or_default()))
}

fn contracts(doc: &Html) -> Vec<Contract> {
    let Ok(card) = Selector::parse("app-contract") else {
        return Vec::new();
    };
    doc.select(&card).filter_map(contract).collect()
}

fn contract(card: ElementRef<'_>) -> Option<Contract> {
    let name = select_text(card, "div.font-semibold.leading-tight")?;
    Some(Contract {
        slug: slugify(&name),
        description: select_text(card, "div.opacity-75"),
        difficulty: None,
        name,
    })
}

fn upgrades(doc: &Html) -> Vec<Upgrade> {
    let Ok(node) = Selector::parse("app-upgrade-node") else {
        return Vec::new();
    };
    doc.select(&node).filter_map(upgrade).collect()
}

fn upgrade(node: ElementRef<'_>) -> Option<Upgrade> {
    let name = select_text(node, "div.capitalize")?;
    Some(Upgrade { name, cost: None, requirements: None })
}
