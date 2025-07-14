SELECT
    user_id,
    higher_or_lower_score
FROM
    gambling_stats
WHERE
    ($1 IS TRUE)
    OR (user_id = ANY ($2))
ORDER BY
    higher_or_lower_score DESC
LIMIT
    $3
OFFSET
    $4