SELECT
    g.user_id,
    g.coins,
    g.daily AS "daily: jiff_sqlx::Date",
    gm.prestige,
    l.level
FROM
    gambling g
JOIN gambling_mine gm
    ON g.user_id = gm.user_id
JOIN levels l
    ON g.user_id = l.user_id
WHERE
    g.user_id = $1
