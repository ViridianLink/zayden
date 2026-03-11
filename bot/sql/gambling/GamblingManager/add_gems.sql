INSERT INTO
gambling (user_id, gems)
VALUES
($1, $2) ON CONFLICT (user_id)
DO
UPDATE
    SET
        gems = gambling.gems + $2

