WITH
    user_leaderboard AS (
        SELECT
            user_id,
            ROW_NUMBER() OVER (
                ORDER BY
                    higher_or_lower_score DESC
            ) as rank
        FROM
            gambling_stats
        WHERE
            ($1 IS TRUE)
            OR (user_id = ANY ($2))
    )
SELECT
    rank
FROM
    user_leaderboard
WHERE
    user_id = $3;