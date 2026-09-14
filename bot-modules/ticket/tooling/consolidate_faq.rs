use std::env;
use std::error::Error;
use std::time::Duration;

use sqlx::PgPool;
use ticket::FaqArticle;
use ticket::faq::reconcile::{Outcome, Reconciler, Subject, candidates};
use zayden_app::config::BotConfig;

const BUSY_ATTEMPTS: usize = 4;
const BUSY_BACKOFF: Duration = Duration::from_secs(30);

const GUILD_ID: i64 = 1_120_465_621_554_040_942;
const CONTEXT: &str = "Servers@Home, a community for people running their own \
home servers: Docker and Docker Compose, Proxmox, Unraid and TrueNAS, media \
servers such as Plex and Jellyfin, the *arr stack (Sonarr, Radarr, Prowlarr), \
usenet and torrent downloaders, VPNs and reverse proxies.";

type BoxError = Box<dyn Error>;

#[derive(Default)]
struct Tally {
    kept: usize,
    merged: usize,
    discarded: usize,
    unresolved: usize,
    failed: usize,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), BoxError> {
    let dry_run = env::args().any(|arg| arg == "--dry-run");
    let guild = GUILD_ID;

    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    let config = BotConfig::load(&pool).await?;
    let reconciler = Reconciler::new(
        &config.ai_provider_key,
        &config.ai_api_endpoint,
        &config.ai_model_structured,
        &config.ai_model_pro,
    )?
    .with_context(CONTEXT);

    let mut ids = FaqArticle::list(&pool, guild, i64::MAX)
        .await?
        .into_iter()
        .map(|article| article.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();

    println!(
        "reviewing {} articles{}",
        ids.len(),
        if dry_run { " (dry run)" } else { "" }
    );

    let mut tally = Tally::default();

    for id in ids {
        let Some(article) = FaqArticle::get(&pool, guild, id).await? else {
            continue;
        };

        let existing = candidates(&pool, guild, article.as_new(), Some(id)).await?;

        if existing.is_empty() {
            tally.kept += 1;
            continue;
        }

        let subject = Subject { article: article.as_new(), dated: article.dated() };

        let outcome = match reconcile(&reconciler, subject, &existing).await {
            Ok(outcome) => outcome,
            Err(e) => {
                eprintln!("failed #{id} {}: {e}", article.title);
                tally.failed += 1;
                continue;
            },
        };

        match outcome {
            Outcome::Create => tally.kept += 1,
            Outcome::Unresolved { reason } => {
                println!("unresolved #{id} {}: {reason}", article.title);
                tally.unresolved += 1;
            },
            Outcome::Discard { target, reason } => {
                println!(
                    "discard #{id} {} (covered by #{target}): {reason}",
                    article.title
                );

                if !dry_run {
                    FaqArticle::delete(&pool, guild, id).await?;
                }

                tally.discarded += 1;
            },
            Outcome::Merge { target, article: merged, reason } => {
                println!(
                    "merge #{id} {} into #{target} as {}: {reason}",
                    article.title, merged.title
                );

                if !dry_run {
                    let mut tx = pool.begin().await?;

                    if FaqArticle::merge(&mut *tx, guild, target, merged.as_new())
                        .await?
                        .is_some()
                    {
                        FaqArticle::delete(&mut *tx, guild, id).await?;
                    }

                    tx.commit().await?;
                }

                tally.merged += 1;
            },
        }
    }

    println!(
        "done: {} kept, {} merged, {} discarded, {} unresolved, {} failed",
        tally.kept, tally.merged, tally.discarded, tally.unresolved, tally.failed
    );

    Ok(())
}

async fn reconcile(
    reconciler: &Reconciler,
    subject: Subject<'_>,
    existing: &[FaqArticle],
) -> Result<Outcome, ai::Error> {
    let mut attempt = 1;

    loop {
        match reconciler.reconcile(subject, existing).await {
            Err(e) if e.is_transient() && attempt < BUSY_ATTEMPTS => {
                eprintln!(
                    "provider unavailable ({e}), retrying in {BUSY_BACKOFF:?}"
                );
                tokio::time::sleep(BUSY_BACKOFF).await;
                attempt += 1;
            },
            result => return result,
        }
    }
}
