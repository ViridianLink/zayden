//! Covers each `ComponentsV2` embed builder in `src/embeds.rs`, including a
//! "missing data" case per entity to confirm graceful degradation (renders
//! "unavailable" rather than omitting the section or panicking).

use jiff::civil::Weekday;
use marathon::embeds;
use marathon::model::{
    Ability,
    Attachment,
    AttachmentSlot,
    BuildRecipe,
    Contract,
    Cradle,
    CradleNode,
    Faction,
    Location,
    LootRoom,
    MapStatus,
    MarathonMap,
    MetaEntry,
    Poi,
    RotationWindow,
    Runner,
    Schedule,
    Stat,
    Upgrade,
    Weapon,
};

fn component_json(component: &serenity::all::CreateComponent<'_>) -> String {
    serde_json::to_string(component).unwrap_or_default()
}

#[test]
fn weapon_component_renders_full_data() {
    let weapon = Weapon {
        slug: "d54-battle-pistol".to_string(),
        name: "D54 Battle Pistol".to_string(),
        weapon_type: Some("Pistol".to_string()),
        ammo_type: Some("Light".to_string()),
        damage: Some("16".to_string()),
        fire_rate: Some("1140 RPM".to_string()),
        magazine_size: Some("21".to_string()),
        reload_speed: Some("2.69s".to_string()),
        range: Some("29m".to_string()),
        description: Some("A reliable sidearm.".to_string()),
        thumbnail_url: Some("https://example.com/d54.png".to_string()),
        stats: vec![Stat { name: "Recoil".to_string(), value: "Low".to_string() }],
        attachment_slots: vec![AttachmentSlot {
            slot: "Muzzle".to_string(),
            attachment: Some(Attachment {
                slug: "suppressor".to_string(),
                name: "Suppressor".to_string(),
                slot: Some("Muzzle".to_string()),
                effect: Some("Reduces sound radius.".to_string()),
                compatible_weapons: vec!["D54 Battle Pistol".to_string()],
            }),
        }],
    };

    let json = component_json(&embeds::weapon_component(&weapon));

    assert!(json.contains("D54 Battle Pistol"));
    assert!(json.contains("1140 RPM"));
    assert!(json.contains("Suppressor"));
    assert!(json.contains("Reduces sound radius."));
    assert!(json.contains("https://example.com/d54.png"));
}

#[test]
fn weapon_component_degrades_when_data_missing() {
    let weapon = Weapon {
        slug: "unknown".to_string(),
        name: "Unknown Weapon".to_string(),
        ..Default::default()
    };

    let json = component_json(&embeds::weapon_component(&weapon));

    assert!(json.contains("Unknown Weapon"));
    assert!(json.contains("unavailable"));
}

#[test]
fn attachment_component_degrades_when_compatible_weapons_missing() {
    let attachment = Attachment {
        slug: "suppressor".to_string(),
        name: "Suppressor".to_string(),
        ..Default::default()
    };

    let json = component_json(&embeds::attachment_component(&attachment));

    assert!(json.contains("Suppressor"));
    assert!(json.contains("unavailable"));
}

#[test]
fn runner_component_renders_abilities_and_degrades_without_cores() {
    let runner = Runner {
        slug: "assassin".to_string(),
        name: "Assassin".to_string(),
        role: Some("Infiltrator".to_string()),
        description: None,
        portrait_url: None,
        abilities: vec![Ability {
            ability_type: Some("Prime Ability".to_string()),
            name: "Smoke Screen".to_string(),
            description: Some("Deploys a smoke disc.".to_string()),
            cooldown_seconds: Some(163),
        }],
        cores: Vec::new(),
        stats: Vec::new(),
    };

    let json = component_json(&embeds::runner_component(&runner));

    assert!(json.contains("Smoke Screen"));
    assert!(json.contains("163s cooldown"));
    // No cores were supplied, so the "Cores" section must not be rendered at all.
    assert!(!json.contains("Cores"));
}

#[test]
fn cradle_component_degrades_when_nodes_missing() {
    let cradle = Cradle {
        description: Some("Persistent stat system.".to_string()),
        nodes: Vec::new(),
    };

    let json = component_json(&embeds::cradle_component(&cradle));

    assert!(json.contains("Persistent stat system."));
    assert!(json.contains("unavailable"));
}

#[test]
fn cradle_component_renders_nodes() {
    let cradle = Cradle {
        description: None,
        nodes: vec![CradleNode {
            name: "Knife Damage".to_string(),
            description: Some("Increases melee damage.".to_string()),
        }],
    };

    let json = component_json(&embeds::cradle_component(&cradle));

    assert!(json.contains("Knife Damage"));
    assert!(json.contains("Increases melee damage."));
}

#[test]
fn build_component_renders_gear_and_notes() {
    let build = BuildRecipe {
        slug: "wallzer-greed-is-good-thief".to_string(),
        name: "Greed Is Good".to_string(),
        shell: Some("Thief".to_string()),
        cradle_focus: Some("Knife Damage".to_string()),
        gear: vec!["Combat Knife".to_string()],
        notes: Some("Best used at close range.".to_string()),
    };

    let json = component_json(&embeds::build_component(&build));

    assert!(json.contains("Greed Is Good"));
    assert!(json.contains("Thief"));
    assert!(json.contains("Combat Knife"));
    assert!(json.contains("Best used at close range."));
}

#[test]
fn build_component_degrades_when_gear_missing() {
    let build =
        BuildRecipe { name: "Mystery Build".to_string(), ..Default::default() };

    let json = component_json(&embeds::build_component(&build));

    assert!(json.contains("Mystery Build"));
    assert!(json.contains("unavailable"));
}

#[test]
fn map_component_renders_all_sections_and_status() {
    let map = MarathonMap {
        slug: "perimeter".to_string(),
        name: "Perimeter".to_string(),
        status: Some(MapStatus::Duo),
        map_image_url: Some("https://example.com/perimeter-map.png".to_string()),
        pois: vec![Poi { name: "Lighthouse".to_string(), description: None }],
        extractions: vec![Location {
            name: "North Dock".to_string(),
            description: Some("Boat extraction.".to_string()),
        }],
        event_spawns: vec![Location {
            name: "Vault Room".to_string(),
            description: None,
        }],
        keycard_rooms: vec![LootRoom {
            name: "Armory".to_string(),
            location_hint: Some("Behind the lighthouse.".to_string()),
        }],
    };

    let json = component_json(&embeds::map_component(&map));

    assert!(json.contains("Perimeter"));
    assert!(json.contains("Duo"));
    assert!(json.contains("Lighthouse"));
    assert!(json.contains("North Dock"));
    assert!(json.contains("Vault Room"));
    assert!(json.contains("Armory"));
    assert!(json.contains("Behind the lighthouse."));
    assert!(json.contains("https://example.com/perimeter-map.png"));
}

#[test]
fn map_component_degrades_when_status_and_sections_missing() {
    let map = MarathonMap { name: "Mystery Map".to_string(), ..Default::default() };

    let json = component_json(&embeds::map_component(&map));

    assert!(json.contains("Mystery Map"));
    assert!(json.contains("unavailable"));
}

#[test]
fn faction_component_renders_contracts_and_upgrades() {
    let faction = Faction {
        slug: "loyalists".to_string(),
        name: "Loyalists".to_string(),
        priority_contracts: vec![Contract {
            slug: "recover-data".to_string(),
            name: "Recover Data".to_string(),
            description: Some("Retrieve the data drive.".to_string()),
            difficulty: Some("Hard".to_string()),
        }],
        upgrades: vec![Upgrade {
            name: "Extended Mags".to_string(),
            cost: Some("500 Scrip".to_string()),
            requirements: None,
        }],
    };

    let json = component_json(&embeds::faction_component(&faction));

    assert!(json.contains("Recover Data"));
    assert!(json.contains("Hard"));
    assert!(json.contains("Extended Mags"));
    assert!(json.contains("500 Scrip"));
}

#[test]
fn faction_component_degrades_when_data_missing() {
    let faction =
        Faction { name: "Unknown Faction".to_string(), ..Default::default() };

    let json = component_json(&embeds::faction_component(&faction));

    assert!(json.contains("Unknown Faction"));
    assert!(json.contains("unavailable"));
}

#[test]
fn meta_component_renders_entries() {
    let entries = vec![MetaEntry {
        item: "Miseriah Shotgun".to_string(),
        tier: Some("S".to_string()),
        note: Some("Dominant this season.".to_string()),
    }];

    let json = component_json(&embeds::meta_component(&entries));

    assert!(json.contains("Miseriah Shotgun"));
    assert!(json.contains("Dominant this season."));
}

#[test]
fn meta_component_degrades_when_empty() {
    let json = component_json(&embeds::meta_component(&[]));

    assert!(json.contains("unavailable"));
}

#[test]
fn schedule_component_renders_windows_and_pool() {
    let schedule = Schedule {
        ranked_window: RotationWindow {
            start_weekday: Weekday::Sunday,
            start_hour_pt: 10,
            end_weekday: Weekday::Thursday,
            end_hour_pt: 10,
            active: true,
        },
        cryo_window: RotationWindow {
            start_weekday: Weekday::Thursday,
            start_hour_pt: 10,
            end_weekday: Weekday::Sunday,
            end_hour_pt: 10,
            active: false,
        },
        duo_map_pool: vec!["Perimeter".to_string(), "Outpost".to_string()],
        weekly_game_mode: None,
    };

    let json = component_json(&embeds::schedule_component(&schedule));

    assert!(json.contains("Ranked"));
    assert!(json.contains("Cryo Archive"));
    assert!(json.contains("Sunday"));
    assert!(json.contains("Thursday"));
    assert!(json.contains("Perimeter"));
    assert!(json.contains("Outpost"));
    // No weekly game mode was supplied - the section must not render at all.
    assert!(!json.contains("Weekly Game Mode"));
}

#[test]
fn schedule_component_degrades_when_pool_missing() {
    let schedule = Schedule {
        ranked_window: RotationWindow {
            start_weekday: Weekday::Sunday,
            start_hour_pt: 10,
            end_weekday: Weekday::Thursday,
            end_hour_pt: 10,
            active: true,
        },
        cryo_window: RotationWindow {
            start_weekday: Weekday::Thursday,
            start_hour_pt: 10,
            end_weekday: Weekday::Sunday,
            end_hour_pt: 10,
            active: false,
        },
        duo_map_pool: Vec::new(),
        weekly_game_mode: None,
    };

    let json = component_json(&embeds::schedule_component(&schedule));

    assert!(json.contains("unavailable"));
}
