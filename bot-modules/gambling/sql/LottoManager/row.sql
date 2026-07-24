SELECT
    g.user_id,
    g.coins,
    COALESCE(i.quantity, 0) AS quantity
FROM
    gambling g
LEFT JOIN gambling_inventory i
    ON
        g.user_id = i.user_id
        AND i.item_id = $2
WHERE
    g.user_id = $1

