use super::draft::{EmojiKey, LoadoutDraft};
use super::record::{GEAR_HEADING, SUBCLASS_HEADING};
use crate::DraftError;

pub const MAX_COMPONENTS: usize = 40;
pub const MAX_TEXT: usize = 4000;
const EMOJI_WRAPPER: usize = "<::>".len() + 20;

#[must_use]
pub const fn components(tags: usize, weapons: usize, armour: usize) -> usize {
    const FIXED: usize = 16;
    let spacer = if weapons > 0 { 1 } else { 0 };
    FIXED + tags + 3 * (weapons + armour) + spacer
}

#[must_use]
pub fn component_warning(draft: &LoadoutDraft) -> Option<DraftError> {
    let needed =
        components(draft.tags.len(), draft.weapons.len(), draft.armour.len());
    (needed > MAX_COMPONENTS)
        .then_some(DraftError::TooManyComponents { needed, max: MAX_COMPONENTS })
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

const fn emoji(name: &str) -> usize {
    name.len() + EMOJI_WRAPPER
}

fn emojis<'a>(keys: impl IntoIterator<Item = &'a EmojiKey>) -> usize {
    keys.into_iter().map(|k| emoji(k.as_str())).sum()
}

#[must_use]
pub fn text(draft: &LoadoutDraft) -> usize {
    let class = chars(&draft.class.to_string());
    let element = chars(&draft.element.to_string());

    let heading1 = "-#   Build".len() + element + class;
    let video = draft
        .video_url
        .as_deref()
        .map_or(0, |url| " • [Video Guide]()".chars().count() + chars(url));
    let heading2 = "#   •    •  \nBy ".chars().count()
        + class
        + chars(&draft.super_name)
        + chars(&draft.name)
        + chars(&draft.author)
        + video;

    let abilities = [
        &draft.super_emoji,
        &draft.class_ability,
        &draft.jump,
        &draft.melee,
        &draft.grenade,
    ];
    let subclass = "#            \n\nFragments".len()
        + emojis(abilities)
        + emojis(draft.aspects.iter().map(|a| &a.aspect))
        + draft.aspects.len();
    let fragment_keys = draft.aspects.iter().flat_map(|a| &a.fragments);
    let fragments = 1 + emojis(fragment_keys.clone()) + fragment_keys.count();

    let weapons: usize = draft
        .weapons
        .iter()
        .map(|w| {
            "****\n \n#".len()
                + chars(&w.name)
                + emoji(&w.affinity.to_string().to_lowercase())
                + chars(&w.archetype.to_string())
                + emojis(&w.perks)
                + w.perks.len()
        })
        .sum();

    let armour: usize = draft
        .armour
        .iter()
        .map(|a| "****\n#".len() + chars(&a.name) + emojis(&a.mods) + a.mods.len())
        .sum();

    let stats: usize = draft
        .stats
        .iter()
        .map(|(stat, _)| " → `199` ".chars().count() + emoji(&stat.to_string()))
        .sum();
    let how = draft
        .how_it_works
        .as_deref()
        .map_or(0, |h| "\n### HOW IT WORKS\n# ".len() + chars(h));
    let misc = "### Stats Priority\n#\n### ARTIFACT PERKS\n# ".len()
        + stats
        + emojis(&draft.artifact_perks)
        + draft.artifact_perks.len()
        + how;

    heading1
        + heading2
        + chars(SUBCLASS_HEADING)
        + subclass
        + fragments
        + chars(GEAR_HEADING)
        + weapons
        + armour
        + misc
}
