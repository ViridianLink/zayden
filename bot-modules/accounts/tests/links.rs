use accounts::links::AccountLink;
use accounts::provider::Provider;
use sqlx::PgPool;

const FIRST_ID: &str = "76561197960287930";
const SECOND_ID: &str = "76561197960287931";

#[sqlx::test(migrations = "../../migrations")]
async fn an_upserted_link_can_be_selected(pool: PgPool) {
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, FIRST_ID).await.unwrap();

    let link =
        AccountLink::select(&pool, 1, Provider::Steam).await.unwrap().unwrap();

    assert_eq!(link.discord_id, 1);
    assert_eq!(link.provider, Provider::Steam);
    assert_eq!(link.external_id, FIRST_ID);
    assert_eq!(link.steam_id().unwrap().get().to_string(), FIRST_ID);
    assert!(AccountLink::select(&pool, 2, Provider::Steam).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn relinking_replaces_the_external_id(pool: PgPool) {
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, FIRST_ID).await.unwrap();
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, SECOND_ID)
        .await
        .unwrap();

    let links = AccountLink::list_for_user(&pool, 1).await.unwrap();

    assert_eq!(links.len(), 1);
    assert_eq!(links.first().unwrap().external_id, SECOND_ID);
}

#[sqlx::test(migrations = "../../migrations")]
async fn linking_an_account_already_linked_elsewhere_transfers_it(pool: PgPool) {
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, FIRST_ID).await.unwrap();
    AccountLink::upsert_transfer(&pool, 2, Provider::Steam, FIRST_ID).await.unwrap();

    assert!(AccountLink::select(&pool, 1, Provider::Steam).await.unwrap().is_none());
    let link =
        AccountLink::select(&pool, 2, Provider::Steam).await.unwrap().unwrap();
    assert_eq!(link.external_id, FIRST_ID);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_reports_whether_a_link_existed(pool: PgPool) {
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, FIRST_ID).await.unwrap();

    assert!(AccountLink::delete(&pool, 1, Provider::Steam).await.unwrap());
    assert!(!AccountLink::delete(&pool, 1, Provider::Steam).await.unwrap());
    assert!(AccountLink::select(&pool, 1, Provider::Steam).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn listing_returns_only_the_users_links(pool: PgPool) {
    AccountLink::upsert_transfer(&pool, 1, Provider::Steam, FIRST_ID).await.unwrap();
    AccountLink::upsert_transfer(&pool, 2, Provider::Steam, SECOND_ID)
        .await
        .unwrap();

    let links = AccountLink::list_for_user(&pool, 1).await.unwrap();

    assert_eq!(links.len(), 1);
    assert_eq!(links.first().unwrap().discord_id, 1);
    assert!(AccountLink::list_for_user(&pool, 3).await.unwrap().is_empty());
}
