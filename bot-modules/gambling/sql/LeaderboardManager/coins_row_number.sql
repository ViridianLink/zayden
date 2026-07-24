WITH
numbered_users AS (
    SELECT
        user_id,
        ROW_NUMBER() OVER (
            ORDER BY
                coins DESC
        ) AS rn
    FROM
        gambling
    WHERE
        ($1 IS TRUE)
        OR (user_id = ANY($2))
)

SELECT rn
FROM
    numbered_users
WHERE
    user_id = $3

