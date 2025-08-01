SELECT
    g.id,
    g.coins,
    g.gems,
    g.stamina,
    (
        SELECT
            jsonb_agg(
                jsonb_build_object('quantity', inv.quantity, 'item_id', inv.item_id)
            )
        FROM
            gambling_inventory inv
        WHERE
            inv.user_id = g.id
    ) as "inventory: Json<Vec<GamblingItem>>",
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
    LEFT JOIN gambling_inventory i on g.id = i.id
    LEFT JOIN gambling_mine m on g.id = m.id
WHERE
    g.id = $1;