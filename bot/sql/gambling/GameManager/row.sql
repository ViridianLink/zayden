SELECT
    g.user_id,
    g.coins,
    g.gems,

    COALESCE(l.level, 0) AS level,

    COALESCE(m.prestige, 0) AS prestige
FROM
    gambling AS g
LEFT JOIN
    levels AS l
    ON g.user_id = l.user_id
LEFT JOIN
    gambling_mine AS m
    ON g.user_id = m.user_id
WHERE
    g.user_id = $1;

