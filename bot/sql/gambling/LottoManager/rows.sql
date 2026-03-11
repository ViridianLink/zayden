SELECT
    g.user_id,
    g.coins,
    i.quantity AS quantity
FROM
    gambling g
LEFT JOIN gambling_inventory
    i ON g.user_id = i.user_id
AND i.item_id = $1

