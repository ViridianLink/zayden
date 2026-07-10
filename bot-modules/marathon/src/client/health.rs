use tracing::warn;

use crate::model::{Faction, MarathonMap, Runner, Weapon};
use crate::source::SourceId;

pub(super) trait SourceData {
    fn is_degraded(&self) -> bool;
}

impl SourceData for Weapon {
    fn is_degraded(&self) -> bool {
        self.stats.is_empty()
            && self.attachment_slots.is_empty()
            && self.weapon_type.is_none()
            && self.ammo_type.is_none()
            && self.damage.is_none()
            && self.fire_rate.is_none()
            && self.magazine_size.is_none()
            && self.reload_speed.is_none()
            && self.range.is_none()
    }
}

impl SourceData for Runner {
    fn is_degraded(&self) -> bool {
        self.stats.is_empty()
            && self.abilities.is_empty()
            && self.cores.is_empty()
            && self.role.is_none()
    }
}

impl SourceData for Faction {
    fn is_degraded(&self) -> bool {
        self.priority_contracts.is_empty() && self.upgrades.is_empty()
    }
}

impl SourceData for MarathonMap {
    fn is_degraded(&self) -> bool {
        self.pois.is_empty()
            && self.extractions.is_empty()
            && self.event_spawns.is_empty()
            && self.keycard_rooms.is_empty()
    }
}

pub(super) fn flag_degraded_sources<T: SourceData>(
    candidates: &[(SourceId, T)],
    entity: &str,
    slug: &str,
) {
    if !candidates.iter().any(|(_, value)| !value.is_degraded()) {
        return;
    }

    for (source, _) in candidates.iter().filter(|(_, value)| value.is_degraded()) {
        warn!(
            %source,
            %slug,
            entity,
            "source parsed to an empty result while others returned data; its page \
             structure may have changed"
        );
    }
}
