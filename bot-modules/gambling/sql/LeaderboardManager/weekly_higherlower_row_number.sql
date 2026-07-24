WITH
    numbered_users AS (
        SELECT
            user_id,
            ROW_NUMBER() OVER (
                ORDER BY
                    weekly_higher_or_lower_score DESC
            ) as rn
        FROM
            gambling_stats
        WHERE
            (
                ($1 IS TRUE)
                OR (user_id = ANY ($2))
            )
            AND weekly_higher_or_lower_score > 0
    )
SELECT
    rn
FROM
    numbered_users
WHERE
    user_id = $3