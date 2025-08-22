INSERT INTO
    lfg_fireteam (post, user_id, alternative)
VALUES
    ($1, $2, $3) ON CONFLICT (post, user_id)
DO
UPDATE
SET
    alternative = EXCLUDED.alternative