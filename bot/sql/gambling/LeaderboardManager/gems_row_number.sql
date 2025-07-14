WITH
    numbered_users AS (
        SELECT
            id,
            ROW_NUMBER() OVER (
                ORDER BY
                    gems DESC
            ) as rn
        FROM
            gambling
        WHERE
            ($1 IS TRUE)
            OR (id = ANY ($2))
    )
SELECT
    rn
FROM
    numbered_users
WHERE
    id = $3