use leptos::prelude::*;
use leptos_meta::{Title, provide_meta_context};
use leptos_router::components::{
    A,
    Outlet,
    ParentRoute,
    Redirect,
    Route,
    Router,
    Routes,
};
use leptos_router::path;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct GuildInfo {
    id: String,
    name: String,
    icon: Option<String>,
}

/// Server function that retrieves the list of Discord guilds the current user
/// can manage (has `MANAGE_GUILD` or `ADMINISTRATOR` permission bits).
///
/// Returns a redirect to `/login` on the server side when the session is
/// missing or expired, and propagates the error to the caller so the component
/// can display a sensible message on the client side.
#[server]
async fn list_manageable_guilds() -> Result<Vec<GuildInfo>, ServerFnError> {
    use leptos_axum::extract;
    use reqwest::Client;
    use sqlx::{PgPool, Row};
    use tower_cookies::Cookies;

    #[derive(serde::Deserialize)]
    struct DiscordGuild {
        id: String,
        name: String,
        icon: Option<String>,
        permissions: String,
    }

    const MANAGE_GUILD: u64 = 0x20;
    const ADMINISTRATOR: u64 = 0x08;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };
    let Some(http) = use_context::<Client>() else {
        return Err(ServerFnError::ServerError("missing HTTP client".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        leptos_axum::redirect("/login");
        return Err(ServerFnError::ServerError("unauthenticated".to_string()));
    };

    let access_token: String = match sqlx::query(
        "SELECT discord_access_token FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(r)) => r.get("discord_access_token"),
        Ok(None) => {
            leptos_axum::redirect("/login");
            return Err(ServerFnError::ServerError("unauthenticated".to_string()));
        },
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let resp = match http
        .get("https://discord.com/api/v10/users/@me/guilds")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    if !resp.status().is_success() {
        return Err(ServerFnError::ServerError(format!(
            "Discord API returned {}",
            resp.status()
        )));
    }

    let all_guilds: Vec<DiscordGuild> = match resp.json().await {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let guilds = all_guilds
        .into_iter()
        .filter(|g| {
            g.permissions
                .parse::<u64>()
                .is_ok_and(|p| p & ADMINISTRATOR != 0 || p & MANAGE_GUILD != 0)
        })
        .map(|g| GuildInfo { id: g.id, name: g.name, icon: g.icon })
        .collect();

    Ok(guilds)
}

/// Server function that checks whether the current request carries a valid
/// `session` cookie.  On the server, if the session is valid, it also calls
/// `leptos_axum::redirect("/guilds")` so that the HTTP response becomes a 302
/// before any page HTML is sent to the client.
#[server]
async fn check_session() -> Result<bool, ServerFnError> {
    use leptos_axum::extract;
    use sqlx::PgPool;
    use tower_cookies::Cookies;

    let Some(pool) = use_context::<PgPool>() else {
        return Err(ServerFnError::ServerError("missing database pool".to_string()));
    };

    let cookies: Cookies = match extract().await {
        Ok(c) => c,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Some(token) = cookies.get("session").map(|c| c.value().to_owned()) else {
        return Ok(false);
    };

    match sqlx::query(
        "SELECT 1 FROM web_sessions WHERE token = $1 AND expires_at > now()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    {
        Ok(row) => {
            let logged_in = row.is_some();
            if logged_in {
                leptos_axum::redirect("/guilds");
            }
            Ok(logged_in)
        },
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[cfg(feature = "ssr")]
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos::hydration::{AutoReload, HydrationScripts};
    use leptos_meta::MetaTags;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[expect(
    clippy::must_use_candidate,
    reason = "Leptos component; return value is consumed by the view system"
)]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=LoginPage/>
                <ParentRoute path=path!("/") view=Layout>
                    <Route path=path!("") view=Home/>
                    <Route path=path!("guilds") view=GuildListPage/>
                    <Route path=path!("guild/:id/settings") view=GuildSettingsPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// Top nav-bar + collapsible sidebar wrapping the active child route via
/// `<Outlet/>`.
#[component]
fn Layout() -> impl IntoView {
    view! {
        <div class="layout">
            <Title text="Zayden Dashboard"/>
            <NavBar/>
            <div class="layout-body">
                <Sidebar/>
                <main class="layout-main">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    view! {
        <nav class="navbar">
            <A href="/" attr:class="navbar-brand">"Zayden"</A>
            <div class="navbar-links">
                <A href="/guilds">"Servers"</A>
            </div>
        </nav>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let open = RwSignal::new(true);

    view! {
        <aside class="sidebar">
            <button
                class="sidebar-toggle"
                on:click=move |_| open.update(|v| *v = !*v)
            >
                {move || if open.get() { "←" } else { "→" }}
            </button>
            <Show when=move || open.get()>
                <nav class="sidebar-nav">
                    <A href="/guilds">"Servers"</A>
                </nav>
            </Show>
        </aside>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Dashboard"</h1>
            <p>"Select a server from the sidebar to manage its settings."</p>
        </div>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    // Blocking resource: SSR waits for this before sending HTML.
    // If the session is valid, check_session calls leptos_axum::redirect("/guilds")
    // server-side, so logged-in visitors are redirected before seeing this page.
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <Title text="Sign In — Zayden Dashboard"/>
        // When the resource resolves as logged-in, render <Redirect> which fires
        // the 302 on SSR (via ServerRedirectFunction) and navigates client-side.
        <Suspense fallback=|| ()>
            {move || {
                session.get()
                    .and_then(Result::ok)
                    .filter(|&logged_in| logged_in)
                    .map(|_| view! { <Redirect path="/guilds"/> })
            }}
        </Suspense>
        <div class="login-page">
            <h1>"Sign in to Zayden Dashboard"</h1>
            <p>"Connect your Discord account to manage server settings."</p>
            <a href="/auth/discord">"Sign in with Discord"</a>
        </div>
    }
}

#[component]
fn GuildListPage() -> impl IntoView {
    // Blocking resource: SSR waits for the guild list before sending HTML so
    // the unauthenticated redirect fires before any page content is sent.
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());

    view! {
        <Title text="Servers — Zayden Dashboard"/>
        <div class="page">
            <h1>"Your Servers"</h1>
            <Suspense fallback=|| view! { <p class="loading">"Loading servers\u{2026}"</p> }>
                {move || guilds.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load servers: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(list) if list.is_empty() => view! {
                        <p class="empty">"You manage no servers with this account."</p>
                    }.into_any(),
                    Ok(list) => view! {
                        <ul class="guild-list">
                            {list.into_iter().map(|g| {
                                let icon_url = g.icon.map(|hash| {
                                    format!(
                                        "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                                        g.id, hash,
                                    )
                                });
                                let href = format!("/guild/{}/settings", g.id);
                                view! {
                                    <li class="guild-item">
                                        <A href=href>
                                            {icon_url.map(|url| view! {
                                                <img src=url alt="" class="guild-icon"/>
                                            })}
                                            <span class="guild-name">{g.name}</span>
                                        </A>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn GuildSettingsPage() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Server Settings"</h1>
            <p>"Loading settings\u{2026}"</p>
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"404"</h1>
            <p>"Page not found."</p>
        </div>
    }
}
