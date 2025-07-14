INSERT INTO
    gambling (id, gems)
VALUES
    ($1, $2) ON CONFLICT (id)
DO
UPDATE
SET
    gems = gambling.gems + $2