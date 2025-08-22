WITH
    post AS (
        INSERT INTO
            lfg_posts (
                id,
                owner,
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
            owner = EXCLUDED.owner,
            activity = EXCLUDED.activity,
            start_time = EXCLUDED.start_time,
            description = EXCLUDED.description,
            fireteam_size = EXCLUDED.fireteam_size
        RETURNING
            id
    ),
    delete_fireteam AS (
        DELETE FROM lfg_fireteam
        WHERE
            post = (
                SELECT
                    id
                FROM
                    post
            )
    )
INSERT INTO
    lfg_fireteam (post, user_id, alternative)
SELECT
    (
        SELECT
            id
        FROM
            post
    ),
    user_id,
    FALSE
FROM
    UNNEST($7::bigint[]) AS t (user_id)
UNION ALL
SELECT
    (
        SELECT
            id
        FROM
            post
    ),
    user_id,
    TRUE
FROM
    UNNEST($8::bigint[]) AS t (user_id)