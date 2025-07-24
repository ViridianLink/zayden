SELECT
    user_id,
    weekly_higher_or_lower_score
FROM
    gambling_stats
WHERE
    (
        ($1 IS TRUE)
        OR (user_id = ANY ($2))
    )
    AND weekly_higher_or_lower_score > 0
ORDER BY
    weekly_higher_or_lower_score DESC
LIMIT
    $3
OFFSET
    $4