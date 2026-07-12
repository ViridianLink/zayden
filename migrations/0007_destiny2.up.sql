CREATE TYPE destiny2_class AS ENUM(
    'hunter',
    'titan',
    'warlock'
);

CREATE TYPE destiny2_element AS ENUM(
    'arc',
    'solar',
    'void',
    'strand',
    'stasis',
    'prismatic'
);

CREATE TYPE destiny2_mode AS ENUM(
    'all',
    'pve',
    'pvp'
);

CREATE TYPE destiny2_affinity AS ENUM(
    'kinetic',
    'arc',
    'void',
    'solar',
    'stasis',
    'strand'
);

CREATE TYPE destiny2_weapon_slot AS ENUM(
    'kinetic',
    'energy',
    'power'
);

CREATE TYPE destiny2_archetype AS ENUM(
    'auto_rifle',
    'bow',
    'fusion_rifle',
    'glaive',
    'breech_grenade_launcher',
    'grenade_launcher',
    'hand_cannon',
    'linear_fusion_rifle',
    'machine_gun',
    'rocket_pulse_rifle',
    'pulse_rifle',
    'rocket_launcher',
    'scout_rifle',
    'shotgun',
    'rocket_sidearm',
    'sidearm',
    'smg',
    'sniper_rifle',
    'sword',
    'trace_rifle'
);

CREATE TYPE destiny2_tier AS ENUM(
    's',
    'a',
    'b',
    'c',
    'd',
    'e',
    'f',
    'none'
);

CREATE TYPE destiny2_armour_slot AS ENUM(
    'helmet',
    'arms',
    'chest',
    'legs',
    'class_item'
);

CREATE TYPE destiny2_stat AS ENUM(
    'health',
    'melee',
    'grenade',
    'super',
    'class',
    'weapons'
);

CREATE TABLE destiny2_weapons(
    id serial PRIMARY KEY,
    name text NOT NULL UNIQUE,
    affinity destiny2_affinity NOT NULL,
    archetype destiny2_archetype NOT NULL,
    slot destiny2_weapon_slot,
    icon_url text NOT NULL
);

CREATE TABLE destiny2_perks(
    id serial PRIMARY KEY,
    name text NOT NULL UNIQUE
);

CREATE TABLE destiny2_weapon_perks(
    weapon_id integer NOT NULL REFERENCES destiny2_weapons(id) ON DELETE CASCADE,
    perk_id integer NOT NULL REFERENCES destiny2_perks(id) ON DELETE CASCADE,
    PRIMARY KEY (weapon_id, perk_id)
);

CREATE TABLE destiny2_loadouts(
    id serial PRIMARY KEY,
    name text NOT NULL,
    class destiny2_class NOT NULL,
    element destiny2_element NOT NULL,
    mode destiny2_mode NOT NULL DEFAULT 'all',
    super_name text NOT NULL,
    super_emoji text NOT NULL,
    class_ability text NOT NULL,
    jump text NOT NULL,
    melee text NOT NULL,
    grenade text NOT NULL,
    artifact_name text,
    author text NOT NULL,
    dim_link text NOT NULL,
    video_url text,
    how_it_works text,
    UNIQUE (class, element, name)
);

CREATE TABLE destiny2_loadout_aspects(
    id serial PRIMARY KEY,
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    aspect_emoji text NOT NULL,
    UNIQUE (loadout_id, ordinal)
);

CREATE TABLE destiny2_loadout_aspect_fragments(
    aspect_id integer NOT NULL REFERENCES destiny2_loadout_aspects(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    fragment_emoji text NOT NULL,
    PRIMARY KEY (aspect_id, ordinal)
);

CREATE TABLE destiny2_loadout_weapons(
    id serial PRIMARY KEY,
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    slot_ordinal smallint NOT NULL,
    weapon_id integer NOT NULL REFERENCES destiny2_weapons(id),
    UNIQUE (loadout_id, slot_ordinal)
);

CREATE TABLE destiny2_loadout_weapon_perks(
    loadout_weapon_id integer NOT NULL REFERENCES destiny2_loadout_weapons(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    perk_id integer NOT NULL REFERENCES destiny2_perks(id),
    PRIMARY KEY (loadout_weapon_id, ordinal)
);

CREATE TABLE destiny2_loadout_armour(
    id serial PRIMARY KEY,
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    slot destiny2_armour_slot NOT NULL,
    name text NOT NULL,
    icon_url text NOT NULL,
    UNIQUE (loadout_id, slot)
);

CREATE TABLE destiny2_loadout_armour_mods(
    armour_id integer NOT NULL REFERENCES destiny2_loadout_armour(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    mod_emoji text NOT NULL,
    PRIMARY KEY (armour_id, ordinal)
);

CREATE TABLE destiny2_loadout_stats(
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    stat destiny2_stat NOT NULL,
    value smallint NOT NULL,
    PRIMARY KEY (loadout_id, ordinal)
);

CREATE TABLE destiny2_loadout_tags(
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    tag text NOT NULL,
    PRIMARY KEY (loadout_id, ordinal)
);

CREATE TABLE destiny2_loadout_artifact_perks(
    loadout_id integer NOT NULL REFERENCES destiny2_loadouts(id) ON DELETE CASCADE,
    ordinal smallint NOT NULL,
    perk_emoji text NOT NULL,
    PRIMARY KEY (loadout_id, ordinal)
);

CREATE TABLE destiny2_endgame_weapons(
    id serial PRIMARY KEY,
    name text NOT NULL,
    archetype text NOT NULL,
    affinity destiny2_affinity,
    frame text,
    enhanceable boolean NOT NULL DEFAULT FALSE,
    reserves integer,
    barrel text NOT NULL DEFAULT '',
    magazine text NOT NULL DEFAULT '',
    perk_1 text NOT NULL DEFAULT '',
    perk_2 text NOT NULL DEFAULT '',
    origin_trait text NOT NULL DEFAULT '',
    rank smallint NOT NULL DEFAULT 0,
    tier destiny2_tier NOT NULL DEFAULT 'none',
    tier_colour integer NOT NULL DEFAULT 0,
    notes text,
    icon text
);

CREATE TABLE destiny2_compendium_perks(
    id serial PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name text NOT NULL,
    description text NOT NULL
);

CREATE TABLE destiny2_raid_weapons(
    id serial PRIMARY KEY,
    name text NOT NULL UNIQUE,
    emoji text NOT NULL,
    icon_url text NOT NULL
);

