SELECT
    user_id,
    gems
FROM
    gambling
WHERE
    ($1 IS TRUE)
    OR user_id = ANY($2)
ORDER BY
    gems DESC
LIMIT
    $3
    OFFSET
    $4

