SELECT
    p.id,
    p.owner_id,
    p.activity,
    p.start_time AS "start_time: jiff_sqlx::Timestamp",
    p.description,
    p.fireteam_size,
    COALESCE(
        ARRAY_AGG(f.user_id) FILTER (
            WHERE f.user_id IS NOT NULL AND f.alternative = FALSE
        ),
        ARRAY[]::int8 []
    ) AS "fireteam!",
    COALESCE(
        ARRAY_AGG(f.user_id) FILTER (
            WHERE f.user_id IS NOT NULL AND f.alternative = TRUE
        ),
        ARRAY[]::int8 []
    ) AS "alternatives!",
    m.message_id AS "alt_message?",
    m.channel_id AS "alt_channel?"
FROM lfg_posts p
LEFT JOIN lfg_fireteam f ON f.post_id = p.id
LEFT JOIN lfg_messages m ON p.id = m.post_id
WHERE p.id = $1
GROUP BY p.id, m.message_id, m.channel_id;
