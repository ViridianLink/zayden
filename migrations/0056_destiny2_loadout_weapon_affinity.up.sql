ALTER TABLE destiny2_loadout_weapons
    ADD COLUMN affinity destiny2_affinity;

UPDATE
    destiny2_loadout_weapons lw
SET
    affinity = w.affinity
FROM
    destiny2_weapons w
WHERE
    w.id = lw.weapon_id;

ALTER TABLE destiny2_loadout_weapons
    ALTER COLUMN affinity SET NOT NULL;

