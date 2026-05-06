SELECT
    user_id
FROM
    gambling_stats
ORDER BY
    weekly_higher_or_lower_score DESC
LIMIT
    3;