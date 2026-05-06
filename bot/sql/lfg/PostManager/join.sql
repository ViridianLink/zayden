INSERT INTO
lfg_fireteam (post_id, user_id, alternative)
VALUES
($1, $2, $3) ON CONFLICT (post_id, user_id)
DO
UPDATE
    SET
        alternative = excluded.alternative

