use std::fmt::Write;
use std::sync::Arc;

use serenity::all::{ExecuteWebhook, Http, Webhook};
use serenity::small_fixed_array::FixedString;
use tracing::error;

const ERROR_LOGS: &str = "https://discord.com/api/webhooks/1502635500211535964/z3arVVSEK1JORzPUz-UAl9d4mGblA2X1kS-HwhcB20sZfCRfi_HooMHyp6deVOn6JPZC";
const NORMAL_LOGS: &str = "https://discord.com/api/webhooks/1502635665509056633/BLo49gZk5ECTo9yTQIsN_C9tAsJHyZCw-5pzVcOe7PliYJFJ5FhKLXORKDFdfzSe8F8G";

const fn get_avatar(level: tracing::Level) -> &'static str {
    match level {
        tracing::Level::TRACE | tracing::Level::DEBUG => {
            "https://cdn.discordapp.com/embed/avatars/1.png"
        },
        tracing::Level::INFO => "https://cdn.discordapp.com/embed/avatars/0.png",
        tracing::Level::WARN => "https://cdn.discordapp.com/embed/avatars/3.png",
        tracing::Level::ERROR => "https://cdn.discordapp.com/embed/avatars/4.png",
    }
}

struct Inner {
    http: Arc<Http>,
    bot_name: FixedString<u8>,

    error_logs: Webhook,
    normal_logs: Webhook,
}

#[derive(Clone)]
pub struct WebhookLogger(Arc<Inner>);

impl WebhookLogger {
    pub async fn new(http: Arc<Http>) -> Self {
        let bot_name = http.get_current_user().await.map_or_else(
            |_| FixedString::<u8>::from_static_trunc("Default"),
            |mut u| std::mem::take(&mut u.name),
        );

        let error_webhook =
            Webhook::from_url(&http, ERROR_LOGS).await.expect("URL is static");
        let normal_webhook =
            Webhook::from_url(&http, NORMAL_LOGS).await.expect("URL is static");

        Self(Arc::new(Inner {
            http,
            bot_name,
            error_logs: error_webhook,
            normal_logs: normal_webhook,
        }))
    }

    pub async fn send_log(
        &self,
        level: tracing::Level,
        target: &str,
        message: String,
    ) {
        let inner = &self.0;
        let webook = match level {
            tracing::Level::ERROR | tracing::Level::WARN => &inner.error_logs,
            tracing::Level::INFO => &inner.normal_logs,
            _ => {
                return;
            },
        };

        let webhook_name =
            format!("{}-Webhook [{}] [{target}]", inner.bot_name, level.as_str());
        let avatar = get_avatar(level);

        for chunk in message
            .as_bytes()
            .chunks(2000)
            .map(|b| std::str::from_utf8(b).expect("Message should always be ASCII"))
        {
            let builder = ExecuteWebhook::default()
                .content(chunk)
                .username(&webhook_name)
                .avatar_url(avatar);

            if let Err(err) = webook.execute(&inner.http, false, builder).await {
                error!(target: "webhook_internal", "Failed to send log message: {err:?}\n{chunk}");
            }
        }
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for WebhookLogger {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();

        let mut message = StringVisitor(String::new());
        event.record(&mut message);

        let level = *metadata.level();
        let target = metadata.target();
        if target == "webhook_internal" {
            return;
        }

        let this = self.clone();
        tokio::spawn(async move { this.send_log(level, target, message.0).await });
    }
}

struct StringVisitor(String);

impl tracing::field::Visit for StringVisitor {
    fn record_debug(
        &mut self,
        _field: &tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        write!(self.0, "{value:?}").expect("fmt::Write for String is infallible");
    }

    fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
        self.0.push_str(value);
    }
}
