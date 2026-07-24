SELECT
    user_id,
    goal_id,
    day AS "day: jiff_sqlx::Date",
    progress,
    target
FROM
    gambling_goals
WHERE
    user_id = $1
