SELECT
    g.id,
    (
        g.coins + COALESCE(gi_eggplants.quantity, 0) * $4 + COALESCE(gi_crates.quantity, 0) * $6
    ) AS networth
FROM
    gambling g
    LEFT JOIN gambling_inventory gi_eggplants ON g.id = gi_eggplants.user_id
    AND gi_eggplants.item_id = $3
    LEFT JOIN gambling_inventory gi_crates ON g.id = gi_crates.user_id
    AND gi_crates.item_id = $5
WHERE
    ($1 IS TRUE)
    OR g.id = ANY ($2)
ORDER BY
    networth DESC
LIMIT
    $7
OFFSET
    $8