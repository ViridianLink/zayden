SELECT
    id,
    gems
FROM
    gambling
WHERE
    ($1 IS TRUE)
    OR id = ANY ($2)
ORDER BY
    gems DESC
LIMIT
    $3
OFFSET
    $4