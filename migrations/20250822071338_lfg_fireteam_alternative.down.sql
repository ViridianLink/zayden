-- Add down migration script here
CREATE TABLE
    lfg_alternatives AS
SELECT
    post,
    user_id
FROM
    lfg_fireteam
WHERE
    alternative = TRUE;

DELETE FROM lfg_fireteam
WHERE
    alternative = TRUE;

ALTER TABLE lfg_fireteam
DROP COLUMN alternative;