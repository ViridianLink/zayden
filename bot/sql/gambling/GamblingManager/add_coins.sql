INSERT INTO
    gambling (id, coins)
VALUES
    ($1, $2) ON CONFLICT (id)
DO
UPDATE
SET
    coins = gambling.coins + $2