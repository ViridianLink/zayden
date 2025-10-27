SELECT
    g.id,
    g.coins,
    COALESCE(i.quantity, 0) AS quantity
FROM
    gambling g
    LEFT JOIN gambling_inventory i ON g.id = i.user_id
    AND i.item_id = $2
WHERE
    g.id = $1