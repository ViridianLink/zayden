pub mod judge;
pub mod merge;
mod prompt;

use std::collections::BTreeSet;

use ai::openai::AiClient;
use jiff::Timestamp;
use sqlx::PgPool;
use zayden_app::state::AppState;

use crate::faq::article::{FaqArticle, NewArticle};
use crate::faq::facts;
use crate::faq::reconcile::judge::{Action, Verdict};
use crate::faq::writer::WrittenArticle;

pub const CANDIDATE_LIMIT: i64 = 4;
const JUDGE_ATTEMPTS: usize = 2;
const MERGE_ATTEMPTS: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct Subject<'a> {
    pub article: NewArticle<'a>,
    pub dated: Timestamp,
}

#[derive(Debug)]
pub enum Outcome {
    Create,
    Merge { target: i32, article: WrittenArticle, reason: String },
    Discard { target: i32, reason: String },
    Unresolved { reason: String },
}

#[derive(Debug)]
pub enum Vetted<'a> {
    Create,
    Merge(&'a FaqArticle),
    Discard(&'a FaqArticle),
    Retry(String),
}

pub async fn candidates(
    pool: &PgPool,
    guild_id: i64,
    article: NewArticle<'_>,
    exclude: Option<i32>,
) -> sqlx::Result<Vec<FaqArticle>> {
    let text =
        format!("{} {} {}", article.title, article.summary, article.tags.join(" "));

    FaqArticle::similar(pool, guild_id, &text, exclude, CANDIDATE_LIMIT).await
}

#[must_use]
pub fn vet<'a>(
    verdict: &Verdict,
    existing: &'a [FaqArticle],
    draft_facts: &BTreeSet<String>,
) -> Vetted<'a> {
    if verdict.action == Action::Create {
        return Vetted::Create;
    }

    let Some(target) = verdict
        .target_id
        .and_then(|id| existing.iter().find(|article| article.id == id))
    else {
        return Vetted::Retry(format!(
            "target_id {:?} is not one of the existing articles. Use an id from \
             the list, or choose create.",
            verdict.target_id
        ));
    };

    if verdict.action == Action::Merge {
        return Vetted::Merge(target);
    }

    let lost = facts::missing(draft_facts, &target.content);

    if lost.is_empty() {
        return Vetted::Discard(target);
    }

    Vetted::Retry(format!(
        "discard is not allowed for article {}, because it lacks these details \
         from the new article: {}. Choose merge or create.",
        target.id,
        lost.iter()
            .map(|detail| format!("`{detail}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub struct Reconciler {
    judge: AiClient,
    writer: AiClient,
    context: Option<String>,
}

impl Reconciler {
    pub fn new(
        api_key: &str,
        endpoint: &str,
        judge_model: &str,
        writer_model: &str,
    ) -> Result<Self, ai::Error> {
        Ok(Self {
            judge: AiClient::new(api_key, endpoint, judge_model)?,
            writer: AiClient::new(api_key, endpoint, writer_model)?,
            context: None,
        })
    }

    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn from_app(app: &AppState) -> Result<Self, ai::Error> {
        Self::new(
            &app.ai_provider_key,
            &app.ai_api_endpoint,
            &app.ai_model_structured,
            &app.ai_model_pro,
        )
    }

    pub async fn reconcile(
        &self,
        subject: Subject<'_>,
        existing: &[FaqArticle],
    ) -> Result<Outcome, ai::Error> {
        if existing.is_empty() {
            return Ok(Outcome::Create);
        }

        let draft_facts = facts::literals(subject.article.content);
        let mut feedback = None;

        for _ in 0..JUDGE_ATTEMPTS {
            let verdict = judge::decide(
                &self.judge,
                self.context.as_deref(),
                subject,
                existing,
                feedback.as_deref(),
            )
            .await?;

            match vet(&verdict, existing, &draft_facts) {
                Vetted::Create => return Ok(Outcome::Create),
                Vetted::Discard(target) => {
                    return Ok(Outcome::Discard {
                        target: target.id,
                        reason: verdict.reason,
                    });
                },
                Vetted::Merge(target) => {
                    return self
                        .merge(subject, target, &draft_facts, verdict.reason)
                        .await;
                },
                Vetted::Retry(message) => feedback = Some(message),
            }
        }

        Ok(Outcome::Unresolved { reason: feedback.unwrap_or_default() })
    }

    async fn merge(
        &self,
        subject: Subject<'_>,
        target: &FaqArticle,
        draft_facts: &BTreeSet<String>,
        reason: String,
    ) -> Result<Outcome, ai::Error> {
        let mut required = facts::literals(&target.content);
        required.extend(draft_facts.iter().cloned());

        let mut missing = Vec::new();

        for _ in 0..MERGE_ATTEMPTS {
            let article = merge::write(
                &self.writer,
                self.context.as_deref(),
                subject,
                target,
                &missing,
            )
            .await?;

            missing = facts::missing(&required, &article.markdown)
                .into_iter()
                .map(str::to_owned)
                .collect();

            if article.is_usable() && missing.is_empty() {
                return Ok(Outcome::Merge { target: target.id, article, reason });
            }
        }

        Ok(Outcome::Unresolved {
            reason: format!(
                "merge into article {} kept dropping: {}",
                target.id,
                missing.join(", ")
            ),
        })
    }
}
