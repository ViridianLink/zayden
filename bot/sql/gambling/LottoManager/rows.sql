SELECT
    g.id,
    g.coins,
    i.quantity AS quantity
FROM
    gambling g
    LEFT JOIN gambling_inventory i ON g.id = i.user_id
    AND i.item_id = $1