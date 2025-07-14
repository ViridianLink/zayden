WITH
    user_networths AS (
        SELECT
            g.id,
            (
                g.coins + COALESCE(gi_eggplants.quantity, 0) * $5 + COALESCE(gi_crates.quantity, 0) * $7
            ) AS networth_value
        FROM
            gambling g
            LEFT JOIN gambling_inventory gi_eggplants ON g.id = gi_eggplants.user_id
            AND gi_eggplants.item_id = $4
            LEFT JOIN gambling_inventory gi_crates ON g.id = gi_crates.user_id
            AND gi_crates.item_id = $6
        WHERE
            ($1 IS TRUE)
            OR (g.id = ANY ($2))
    ),
    ranked_users AS (
        SELECT
            id,
            ROW_NUMBER() OVER (
                ORDER BY
                    networth_value DESC
            ) as rn
        FROM
            user_networths
    )
SELECT
    rn
FROM
    ranked_users
WHERE
    id = $3