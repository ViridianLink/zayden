SELECT
    user_id
FROM
    gambling_stats
WHERE
    weekly_higher_or_lower_score > 0
ORDER BY
    weekly_higher_or_lower_score DESC,
    user_id
LIMIT 3;

