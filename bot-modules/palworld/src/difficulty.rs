use crate::model::Pal;

const NO_WILD_PENALTY: i64 = 120;
const RARITY_WEIGHT: i64 = 4;
const NOCTURNAL_PENALTY: i64 = 5;

const OBTAIN_OVERRIDES: &[(&str, i64)] = &[
    // (No overrides yet — the wild-level/rarity signal covers the common cases,
    //  including boss/breeding-only pals via NO_WILD_PENALTY. Add entries here
    //  only for pals the data misrepresents, e.g. raid-boss-only spawns.)
];

#[must_use]
pub fn pal_difficulty(p: &Pal) -> i64 {
    if let Some(&(_, score)) = OBTAIN_OVERRIDES.iter().find(|(key, _)| *key == p.key)
    {
        return score;
    }

    let mut score = p.min_wild_level.unwrap_or(NO_WILD_PENALTY);
    score += p.rarity.unwrap_or(1) * RARITY_WEIGHT;
    if p.nocturnal {
        score += NOCTURNAL_PENALTY;
    }
    score
}

#[must_use]
pub fn pair_difficulty(a: &Pal, b: &Pal) -> (i64, i64) {
    let (da, db) = (pal_difficulty(a), pal_difficulty(b));
    (da.max(db), da + db)
}
