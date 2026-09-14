use std::env;
use std::error::Error;
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sqlx::PgPool;
use ticket::faq::article::PRIMARY_POSITION;
use ticket::faq::scrub::redact;
use ticket::faq::{FaqArticle, NewArticle};
use tokio::time::sleep;

const API: &str = "https://answer.serversatho.me/answer/api/v1";
const GUILD_ID: i64 = 1_120_465_621_554_040_942;
const PAGE_SIZE: usize = 50;
const REQUEST_DELAY: Duration = Duration::from_millis(250);

const MIRROR_FOOTER: &str = "\n---\n";
const IGNORED_TAGS: [&str; 1] = ["discord"];
const MAX_TAGS: usize = 6;
const SUMMARY_LIMIT: usize = 100;
const DUPLICATE_RANK: f32 = 0.9;

type BoxError = Box<dyn Error>;

#[derive(Deserialize)]
struct Envelope<T> {
    code: u16,
    msg: String,
    data: Option<T>,
}

#[derive(Deserialize)]
struct Page<T> {
    count: usize,
    list: Vec<T>,
}

#[derive(Deserialize)]
struct Listing {
    id: String,
}

#[derive(Deserialize)]
struct Question {
    title: String,
    content: String,
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    slug_name: String,
}

#[derive(Deserialize)]
struct Answer {
    content: String,
    accepted: u8,
}

struct Import {
    thread_id: i64,
    title: String,
    summary: String,
    content: String,
    tags: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), BoxError> {
    let dry_run = env::args().any(|arg| arg == "--dry-run");
    let http = Client::new();
    let thread_link = Regex::new(r"discord\.com/channels/(\d+)/(\d+)")?;

    let pool = if dry_run {
        None
    } else {
        Some(PgPool::connect(&env::var("DATABASE_URL")?).await?)
    };

    let ids = question_ids(&http).await?;
    println!("found {} questions", ids.len());

    let (mut inserted, mut existing, mut duplicate, mut skipped) = (0, 0, 0, 0);

    for id in &ids {
        let import = match fetch(&http, &thread_link, id).await {
            Ok(Some(import)) => import,
            Ok(None) => {
                println!("skip {id}: no answer or discord thread");
                skipped += 1;
                continue;
            },
            Err(e) => {
                eprintln!("skip {id}: {e}");
                skipped += 1;
                continue;
            },
        };

        let Some(pool) = &pool else {
            println!(
                "--- {id} (thread {})\n# {}\n> {}\ntags: {:?}\n\n{}\n",
                import.thread_id,
                import.title,
                import.summary,
                import.tags,
                import.content
            );
            continue;
        };

        if let Some(rank) =
            FaqArticle::best_match_rank(pool, GUILD_ID, &import.title).await?
            && rank >= DUPLICATE_RANK
        {
            println!("duplicate {id}: {} (rank {rank})", import.title);
            duplicate += 1;
            continue;
        }

        let article = NewArticle {
            title: &import.title,
            summary: &import.summary,
            content: &import.content,
            category: None,
            tags: &import.tags,
        };

        match FaqArticle::insert_generated(
            pool,
            GUILD_ID,
            import.thread_id,
            PRIMARY_POSITION,
            article,
        )
        .await?
        {
            Some(row) => {
                println!("inserted #{}: {}", row.id, row.title);
                inserted += 1;
            },
            None => {
                println!(
                    "exists {id}: thread {} already has an article",
                    import.thread_id
                );
                existing += 1;
            },
        }
    }

    println!(
        "done: {inserted} inserted, {existing} existing, {duplicate} duplicate, {skipped} skipped"
    );

    Ok(())
}

async fn get<T: DeserializeOwned>(http: &Client, url: &str) -> Result<T, BoxError> {
    sleep(REQUEST_DELAY).await;

    let envelope: Envelope<T> =
        http.get(url).send().await?.error_for_status()?.json().await?;

    match envelope.data {
        Some(data) if envelope.code == 200 => Ok(data),
        _ => Err(format!("{url}: {} {}", envelope.code, envelope.msg).into()),
    }
}

async fn question_ids(http: &Client) -> Result<Vec<String>, BoxError> {
    let mut ids = Vec::new();

    for page in 1.. {
        let url = format!(
            "{API}/question/page?page={page}&page_size={PAGE_SIZE}&order=newest"
        );
        let listing: Page<Listing> = get(http, &url).await?;

        if listing.list.is_empty() {
            break;
        }

        ids.extend(listing.list.into_iter().map(|question| question.id));

        if ids.len() >= listing.count {
            break;
        }
    }

    Ok(ids)
}

async fn fetch(
    http: &Client,
    thread_link: &Regex,
    id: &str,
) -> Result<Option<Import>, BoxError> {
    let question: Question =
        get(http, &format!("{API}/question/info?id={id}")).await?;

    let url =
        format!("{API}/answer/page?question_id={id}&page=1&page_size={PAGE_SIZE}");
    let answers: Page<Answer> = get(http, &url).await?;

    let Some(answer) = answers
        .list
        .iter()
        .find(|answer| answer.accepted == 1)
        .or_else(|| answers.list.first())
    else {
        return Ok(None);
    };

    let Some((body, footer)) = question.content.rsplit_once(MIRROR_FOOTER) else {
        return Ok(None);
    };

    let Some(thread_id) = thread_link
        .captures(footer)
        .filter(|link| {
            link.get(1).is_some_and(|guild| guild.as_str() == GUILD_ID.to_string())
        })
        .and_then(|link| link.get(2))
        .and_then(|thread| thread.as_str().parse().ok())
    else {
        return Ok(None);
    };

    let body = redact(body.trim());
    let solution = redact(answer.content.trim());

    let tags = question
        .tags
        .into_iter()
        .map(|tag| tag.slug_name)
        .filter(|tag| !IGNORED_TAGS.contains(&tag.as_str()))
        .take(MAX_TAGS)
        .collect();

    Ok(Some(Import {
        thread_id,
        title: question.title.trim().to_owned(),
        summary: summarise(&body),
        content: format!(
            "## Problem\n\n{}\n\n## Solution\n\n{}",
            demote_headings(&body),
            demote_headings(&solution)
        ),
        tags,
    }))
}

fn summarise(body: &str) -> String {
    let line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();

    let sentence = line
        .char_indices()
        .find(|&(i, c)| {
            matches!(c, '.' | '?' | '!')
                && line.get(i + 1..).is_some_and(|rest| rest.starts_with(' '))
        })
        .and_then(|(i, _)| line.get(..=i))
        .unwrap_or(line);

    if sentence.chars().count() <= SUMMARY_LIMIT {
        return sentence.to_owned();
    }

    let mut summary = String::with_capacity(SUMMARY_LIMIT);

    for word in sentence.split_whitespace() {
        if summary.chars().count() + word.chars().count() + 4 > SUMMARY_LIMIT {
            break;
        }

        if !summary.is_empty() {
            summary.push(' ');
        }

        summary.push_str(word);
    }

    summary.push_str("...");
    summary
}

fn demote_headings(markdown: &str) -> String {
    let mut in_fence = false;

    markdown
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();

            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
                return line.to_owned();
            }

            let hashes = line.len() - line.trim_start_matches('#').len();

            if in_fence
                || hashes == 0
                || !line.trim_start_matches('#').starts_with(' ')
            {
                return line.to_owned();
            }

            let level = (hashes + 1).clamp(3, 6);
            let text = line.trim_start_matches('#');

            format!("{}{text}", "#".repeat(level))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
