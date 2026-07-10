use std::collections::HashSet;

use super::{BASE, identity, leading_number};
use crate::model::{Stat, Weapon};
use crate::parse::html;

const HERO_ROWS: &str = "div[class*=\"gap-x-8\"] div[class*=\"justify-between\"]";
const DETAIL_ROWS: &str = "div[class*=\"group/sub\"]";

#[must_use]
pub fn marathonmeta_html_to_weapon(slug: &str, rendered: &str) -> Weapon {
    let doc = html::document(rendered);
    let ident = identity(&doc, slug);
    let stats = stats(&doc);

    let lookup = |label: &str| {
        stats.iter().find(|s| s.name == label).map(|s| s.value.as_str())
    };
    let scalar = |label: &str| lookup(label).and_then(leading_number);

    Weapon {
        slug: slug.to_string(),
        name: ident.name,
        weapon_type: ident.category,
        ammo_type: None,
        damage: scalar("Damage").or_else(|| scalar("Dmg/Shot")),
        fire_rate: scalar("Rate of Fire").or_else(|| scalar("RPM")),
        magazine_size: scalar("Magazine"),
        reload_speed: scalar("Reload Speed"),
        range: scalar("Range"),
        description: ident.description,
        thumbnail_url: Some(format!("{BASE}/assets/weapons/{slug}.png")),
        stats,
        attachment_slots: Vec::new(),
    }
}

fn stats(doc: &scraper::Html) -> Vec<Stat> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for css in [HERO_ROWS, DETAIL_ROWS] {
        for (name, value) in html::span_pairs(doc, css).unwrap_or_default() {
            if seen.insert(name.clone()) {
                out.push(Stat { name, value });
            }
        }
    }
    out
}
