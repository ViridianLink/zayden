SELECT
    SUM(quantity)
FROM
    gambling_inventory
WHERE
    item_id = $1