INSERT INTO
gambling (user_id, coins)
VALUES
($1, $2) ON CONFLICT (user_id)
DO
UPDATE
    SET
        coins = gambling.coins + $2

