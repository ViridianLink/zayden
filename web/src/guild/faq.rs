use topcoat::context::Cx;

use super::access::admin_app;
use super::error::{GuildError, server_err};
use super::form::form_args;
use super::parse::{
    parse_answer_max_tokens,
    parse_answer_temperature,
    parse_flag,
    parse_max_results,
    parse_optional,
    parse_wiki_locale,
    parse_wiki_url,
};

form_args! {
    FaqSettingsForm {
        guild,
        enabled,
        auto_triage,
        auto_generate,
        wiki_url,
        wiki_locale,
    }
}

form_args! {
    FaqWikiKeyForm { guild, wiki_api_key, keep_wiki_api_key }
}

form_args! {
    WikiSettingsForm {
        guild,
        enabled,
        auto_triage,
        auto_generate,
        wiki_url,
        wiki_locale,
        max_results,
        answer_max_tokens,
        answer_temperature,
    }
}

pub async fn save_wiki_settings(
    cx: &Cx,
    form: &WikiSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let enabled = parse_flag(&form.enabled);
    let auto_triage = parse_flag(&form.auto_triage);
    let auto_generate = parse_flag(&form.auto_generate);
    let url = parse_wiki_url(&form.wiki_url)?;
    let locale = parse_wiki_locale(&form.wiki_locale);
    let max_results = parse_max_results(&form.max_results);
    let max_tokens = parse_answer_max_tokens(&form.answer_max_tokens);
    let temperature = parse_answer_temperature(&form.answer_temperature);

    app.settings
        .faq
        .update(guild_id, |p| {
            p.enabled = enabled;
            p.auto_triage = auto_triage;
            p.auto_generate = auto_generate;
            p.wiki_url = url;
            p.wiki_locale = locale;
            p.max_results = max_results;
            p.answer_max_tokens = max_tokens;
            p.answer_temperature = temperature;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

form_args! {
    FaqTuningForm { guild, max_results, answer_max_tokens, answer_temperature }
}

pub async fn save_faq_settings(
    cx: &Cx,
    form: &FaqSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let enabled = parse_flag(&form.enabled);
    let auto_triage = parse_flag(&form.auto_triage);
    let auto_generate = parse_flag(&form.auto_generate);
    let url = parse_wiki_url(&form.wiki_url)?;
    let locale = parse_wiki_locale(&form.wiki_locale);

    app.settings
        .faq
        .update(guild_id, |p| {
            p.enabled = enabled;
            p.auto_triage = auto_triage;
            p.auto_generate = auto_generate;
            p.wiki_url = url;
            p.wiki_locale = locale;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_faq_wiki_key(
    cx: &Cx,
    form: &FaqWikiKeyForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let key = parse_optional(&form.wiki_api_key);
    let keep = parse_flag(&form.keep_wiki_api_key);

    if key.is_none() && keep {
        return Ok(());
    }

    app.settings
        .faq
        .update(guild_id, |p| p.wiki_api_key = key)
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_faq_tuning(
    cx: &Cx,
    form: &FaqTuningForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let max_results = parse_max_results(&form.max_results);
    let max_tokens = parse_answer_max_tokens(&form.answer_max_tokens);
    let temperature = parse_answer_temperature(&form.answer_temperature);

    app.settings
        .faq
        .update(guild_id, |p| {
            p.max_results = max_results;
            p.answer_max_tokens = max_tokens;
            p.answer_temperature = temperature;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}
