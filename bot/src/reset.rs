use sqlx::PgPool;

pub async fn reset(pool: &PgPool) {
    // sqlx::query!(
    //     "UPDATE gambling SET cash = 1000, daily = '1970-01-01', work = '1970-01-01 00:00:00.000', gift = '1970-01-01';"
    // ).execute(pool).await.unwrap();

    // sqlx::query!("TRUNCATE TABLE gambling_goals RESTART IDENTITY;")
    //     .execute(pool)
    //     .await
    //     .unwrap();

    sqlx::query!("TRUNCATE TABLE gambling_effects RESTART IDENTITY;")
        .execute(pool)
        .await
        .unwrap();

    // sqlx::query!(
    //     "DELETE FROM gambling_inventory
    //     WHERE item_id != 'luckychip'
    //     AND item_id != 'profit2x'
    //     AND item_id != 'profit5x';"
    // )
    // .execute(pool)
    // .await
    // .unwrap();

    // sqlx::query!("ALTER SEQUENCE gambling_inventory_id_seq RESTART WITH 10;")
    //     .execute(pool)
    //     .await
    //     .unwrap();

    // sqlx::query!(
    //     "UPDATE gambling_mine SET miners = 0, mines = 0, land = 0, countries = 0, continents = 0, planets = 0, solar_systems = 0, galaxies = 0, universes = 0, coal = 0, iron = 0, gold = 0, redstone = 0, lapis = 0, diamonds = 0, emeralds = 0, tech = 0, utility = 0, production = 0;"
    // ).execute(pool).await.unwrap();
}
