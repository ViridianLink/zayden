CREATE FUNCTION notify_loadouts_changed ()
    RETURNS TRIGGER
    AS $$
BEGIN
    PERFORM
        pg_notify('loadouts_changed', '');
    RETURN NULL;
END;
$$
LANGUAGE plpgsql;

DO $$
DECLARE
    tbl text;
BEGIN
    FOREACH tbl IN ARRAY ARRAY['destiny2_loadouts', 'destiny2_loadout_aspects', 'destiny2_loadout_aspect_fragments', 'destiny2_loadout_weapons', 'destiny2_loadout_weapon_perks', 'destiny2_loadout_armour', 'destiny2_loadout_armour_mods', 'destiny2_loadout_stats', 'destiny2_loadout_tags', 'destiny2_loadout_artifact_perks', 'destiny2_weapons', 'destiny2_perks'] LOOP
        EXECUTE format('CREATE TRIGGER %I AFTER INSERT OR UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION notify_loadouts_changed ()', tbl || '_loadouts_notify', tbl);
    END LOOP;
END;
$$;

