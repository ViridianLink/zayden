SELECT
    user_id,
    coins
FROM
    gambling
WHERE
    ($1 IS TRUE)
    OR user_id = ANY($2)
ORDER BY
    coins DESC
LIMIT
    $3
    OFFSET
    $4

