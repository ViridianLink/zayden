-- Add up migration script here
ALTER TABLE lfg_fireteam
ADD COLUMN alternative BOOLEAN NOT NULL DEFAULT FALSE;

INSERT INTO
    lfg_fireteam (post, user_id, alternative)
SELECT
    post,
    user_id,
    TRUE
FROM
    lfg_alternatives;

DROP TABLE lfg_alternatives;