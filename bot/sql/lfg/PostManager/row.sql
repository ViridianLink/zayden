SELECT
    p.id,
    p.owner,
    p.activity,
    p.start_time,
    p.description,
    p.fireteam_size,
    COALESCE(
        array_agg(f.user_id) FILTER (
            WHERE
                f.alternative = false
        ),
        '{}'
    ) AS "fireteam!",
    COALESCE(
        array_agg(f.user_id) FILTER (
            WHERE
                f.alternative = true
        ),
        '{}'
    ) AS "alternatives!",
    m.message AS "alt_message?",
    m.channel AS "alt_channel?"
FROM
    lfg_posts p
    LEFT JOIN lfg_fireteam f ON p.id = f.post
    LEFT JOIN lfg_messages m ON p.id = m.id
WHERE
    p.id = $1
GROUP BY
    p.id,
    m.message,
    m.channel;