use crate::model::Pal;

const PREFIXES: &[&str] =
    &["BOSS_", "GYM_", "PREDATOR_", "RAID_", "SUMMON_", "NPC_"];

const OVERRIDES: &[(&str, &str)] = &[];

fn strip_prefix(codename: &str) -> &str {
    for prefix in PREFIXES {
        if codename
            .get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
            && let Some(rest) = codename.get(prefix.len()..)
        {
            return rest;
        }
    }
    codename
}

#[must_use]
pub fn resolve_species(character_id: &str, pals: &[Pal]) -> Option<String> {
    let base = strip_prefix(character_id.trim());

    if let Some(&(_, key)) =
        OVERRIDES.iter().find(|(c, _)| c.eq_ignore_ascii_case(base))
    {
        return Some(key.to_string());
    }

    if let Some(pal) = pals.iter().find(|p| p.key.eq_ignore_ascii_case(base)) {
        return Some(pal.key.clone());
    }

    pals.iter().find(|p| p.name.eq_ignore_ascii_case(base)).map(|p| p.key.clone())
}
