use std::fmt::Write;
use std::sync::Arc;

use serenity::all::{ExecuteWebhook, Http, Webhook};
use serenity::small_fixed_array::FixedString;
use tracing::error;

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

    error_logs: Option<Webhook>,
    normal_logs: Option<Webhook>,

    handle: tokio::runtime::Handle,
}

#[derive(Clone)]
pub struct WebhookLogger(Arc<Inner>);

async fn resolve_webhook(
    http: &Http,
    url: Option<&str>,
    name: &str,
) -> Option<Webhook> {
    let url = url?;
    match Webhook::from_url(http, url).await {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("webhook_logger: failed to load {name}: {e}");
            None
        },
    }
}

impl WebhookLogger {
    pub async fn new(
        http: Arc<Http>,
        error_log_url: Option<&str>,
        normal_log_url: Option<&str>,
    ) -> Self {
        let bot_name = http.get_current_user().await.map_or_else(
            |_| FixedString::<u8>::from_static_trunc("Default"),
            |mut u| std::mem::take(&mut u.name),
        );

        let error_logs =
            resolve_webhook(&http, error_log_url, "error_log_webhook").await;
        let normal_logs =
            resolve_webhook(&http, normal_log_url, "normal_log_webhook").await;

        Self(Arc::new(Inner {
            http,
            bot_name,
            error_logs,
            normal_logs,
            handle: tokio::runtime::Handle::current(),
        }))
    }

    pub async fn send_log(
        &self,
        level: tracing::Level,
        target: &str,
        message: String,
    ) {
        let inner = &self.0;
        let webhook = match level {
            tracing::Level::ERROR | tracing::Level::WARN => {
                inner.error_logs.as_ref()
            },
            tracing::Level::INFO => inner.normal_logs.as_ref(),
            _ => return,
        };
        let Some(webook) = webhook else {
            return;
        };

        let webhook_name =
            format!("{}-Webhook [{}] [{target}]", inner.bot_name, level.as_str());
        let avatar = get_avatar(level);

        for chunk_bytes in message.as_bytes().chunks(2000) {
            let chunk_cow = String::from_utf8_lossy(chunk_bytes);
            let chunk = chunk_cow.as_ref();
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
        this.0.handle.clone().spawn(async move {
            this.send_log(level, target, message.0).await;
        });
    }
}

struct StringVisitor(String);

impl tracing::field::Visit for StringVisitor {
    fn record_debug(
        &mut self,
        _field: &tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        let _ = write!(self.0, "{value:?}");
    }

    fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
        self.0.push_str(value);
    }
}
