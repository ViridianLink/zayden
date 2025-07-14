WITH
    RankedUsers AS (
        SELECT
            user_id,
            ROW_NUMBER() OVER (
                ORDER BY
                    quantity DESC
            ) as row_num
        FROM
            gambling_inventory
        WHERE
            (
                ($1 IS TRUE)
                OR (user_id = ANY ($2))
            )
            AND item_id = $3
    )
SELECT
    row_num
FROM
    RankedUsers
WHERE
    user_id = $4;