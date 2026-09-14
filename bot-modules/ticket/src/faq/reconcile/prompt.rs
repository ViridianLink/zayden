use std::fmt::Write as _;

use jiff::Timestamp;

use crate::faq::article::NewArticle;

pub(crate) const UNFAMILIAR_DETAILS: &str = "These articles come from real \
support tickets and may describe products, releases, versions, settings, \
features, rules or behaviour newer than your training data. Never treat a \
detail as wrong, mistyped or invented because you do not recognise it, and \
never correct one from your own knowledge: every such detail is real \
information. When two articles disagree, do not decide which is right from what \
you know. The ticket dates show which is newer, and the disagreement itself is \
information worth keeping.";

pub(crate) fn system(sections: &[&str], context: Option<&str>) -> String {
    let mut prompt = sections.join("\n\n");

    if let Some(context) = context.map(str::trim).filter(|c| !c.is_empty()) {
        let _ = write!(prompt, "\n\nAbout this server:\n{context}");
    }

    prompt
}

pub(crate) fn describe(
    prompt: &mut String,
    heading: &str,
    article: NewArticle<'_>,
    dated: Timestamp,
) {
    let _ = write!(
        prompt,
        "{heading} (ticket date {}):\nTitle: {}\nSummary: {}\nTags: {}\nBody:\n{}\n\n",
        dated.strftime("%Y-%m-%d"),
        article.title,
        article.summary,
        if article.tags.is_empty() {
            String::from("(none)")
        } else {
            article.tags.join(", ")
        },
        article.content
    );
}
