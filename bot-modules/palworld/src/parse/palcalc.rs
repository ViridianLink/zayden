use crate::model::{Pal, Stats, Suitability};
use crate::transport::RawPalCalcPal;

#[must_use]
pub fn pal_from_palcalc(raw: RawPalCalcPal) -> Pal {
    let stats = Some(Stats {
        hp: raw.hp,
        attack_melee: raw.attack,
        attack_ranged: raw.attack,
        defense: raw.defense,
        speed_ride: raw.ride_sprint_speed,
        speed_run: raw.run_speed,
        speed_walk: raw.walk_speed,
        stamina: raw.stamina,
        support: 0,
        food: raw.food_amount,
    });

    let suitability = raw
        .work_suitability
        .entries()
        .into_iter()
        .map(|(kind, level)| Suitability { kind: kind.to_string(), level })
        .collect();

    Pal {
        key: raw.internal_name,
        paldex_no: raw.id.pal_dex_no,
        name: raw.name,
        elements: Vec::new(),
        stats,
        suitability,
        drops: Vec::new(),
        partner_skill: None,
        active_skills: Vec::new(),
        description: None,
        genus: None,
        rarity: raw.rarity,
        price: raw.price,
        size: raw.size,
        breeding_rank: Some(raw.breeding_power),
        breeding_order: Some(raw.breeding_power_priority),
        child_eligible: true,
        male_probability: None,
        min_wild_level: raw.min_wild_level,
        max_wild_level: raw.max_wild_level,
        nocturnal: raw.nocturnal,
        image_url: None,
        wiki_url: None,
    }
}
