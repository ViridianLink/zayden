SELECT
    g.user_id,
    g.coins,
    g.gems,
    g.stamina,

    m.miners,
    m.mines,
    m.land,
    m.countries,
    m.continents,
    m.planets,
    m.solar_systems,
    m.galaxies,
    m.universes,
    m.prestige,
    m.coal,
    m.iron,
    m.gold,
    m.redstone,
    m.lapis,
    m.diamonds,
    m.emeralds,
    m.tech,
    m.utility,
    m.production
FROM
    gambling g
LEFT JOIN gambling_mine m ON g.user_id = m.user_id
WHERE
    g.user_id = $1;

