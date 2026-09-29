DO $$
DECLARE
    tbl text;
BEGIN
    FOREACH tbl IN ARRAY ARRAY['destiny2_loadouts', 'destiny2_loadout_aspects', 'destiny2_loadout_aspect_fragments', 'destiny2_loadout_weapons', 'destiny2_loadout_weapon_perks', 'destiny2_loadout_armour', 'destiny2_loadout_armour_mods', 'destiny2_loadout_stats', 'destiny2_loadout_tags', 'destiny2_loadout_artifact_perks', 'destiny2_weapons', 'destiny2_perks'] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS %I ON %I', tbl || '_loadouts_notify', tbl);
    END LOOP;
END;
$$;

DROP FUNCTION IF EXISTS notify_loadouts_changed ();

