WITH
post AS (
    INSERT INTO
    lfg_posts (
        id,
        owner_id,
        activity,
        start_time,
        description,
        fireteam_size
    )
    VALUES
    ($1, $2, $3, $4, $5, $6) ON CONFLICT (id)
    DO
    UPDATE
        SET
            owner_id = excluded.owner_id,
            activity = excluded.activity,
            start_time = excluded.start_time,
            description = excluded.description,
            fireteam_size = excluded.fireteam_size
    RETURNING
        id
),

delete_fireteam AS (
    DELETE FROM lfg_fireteam
    WHERE
        post_id = (
            SELECT id
            FROM
                post
        )
)

INSERT INTO
lfg_fireteam (post_id, user_id, alternative)
SELECT
    (
        SELECT id
        FROM
            post
    ),
    user_id,
    FALSE
FROM
    UNNEST($7::bigint []) AS t (user_id)
UNION ALL
SELECT
    (
        SELECT id
        FROM
            post
    ),
    user_id,
    TRUE
FROM
    UNNEST($8::bigint []) AS t (user_id)

