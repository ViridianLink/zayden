SELECT
    user_id,
    coal,
    iron,
    gold,
    redstone,
    lapis,
    diamonds,
    emeralds,
    tech,
    utility,
    production
FROM
    gambling_mine
WHERE
    user_id = $1