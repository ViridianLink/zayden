use crate::model::Element;

#[must_use]
pub const fn strong_against(attacker: Element) -> &'static [Element] {
    match attacker {
        Element::Fire => &[Element::Grass, Element::Ice],
        Element::Water => &[Element::Fire],
        Element::Electric => &[Element::Water],
        Element::Ground => &[Element::Electric],
        Element::Grass => &[Element::Ground],
        Element::Ice => &[Element::Dragon],
        Element::Dragon => &[Element::Dark],
        Element::Dark => &[Element::Neutral],
        Element::Neutral => &[],
    }
}

#[must_use]
pub fn weak_to(defender: Element) -> Vec<Element> {
    Element::all()
        .into_iter()
        .filter(|&attacker| strong_against(attacker).contains(&defender))
        .collect()
}
