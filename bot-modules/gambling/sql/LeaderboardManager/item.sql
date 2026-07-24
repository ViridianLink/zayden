SELECT
    user_id,
    quantity
FROM
    gambling_inventory
WHERE
    (
        ($1 IS TRUE)
        OR user_id = ANY ($2)
    )
    AND item_id = $3
ORDER BY
    quantity DESC
LIMIT
    $4
OFFSET
    $5