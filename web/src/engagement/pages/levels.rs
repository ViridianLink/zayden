use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{page, path_param};
use topcoat::view::{View, component, suspense, view};

use super::action::path_segment;
use crate::components::skeleton::{SkeletonShape, skeleton};
use crate::engagement::levels::get_leaderboard;
use crate::engagement::{LeaderboardEntry, LeaderboardView};
use crate::shell::GuildId;
use crate::util::server_error_text;

#[page("/guild/{guild_id}/levels")]
pub(super) async fn levels(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let board = LeaderboardView::from_request(cx);

    Ok(view! { levels_page(guild_id: guild_id, board: board) })
}

#[component]
async fn levels_page(guild_id: &str, board: LeaderboardView) -> Result<impl View> {
    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Levels"</h1>
                    <p class="page-lead">
                        "Message-XP rankings. Switch between this server and the global board."
                    </p>
                </div>
                scope_switch(guild_id: guild_id, board: board)
            </div>
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        skeleton(shape: SkeletonShape::Row, count: 10)
                    </div>
                },
                leaderboard(guild_id: guild_id, board: board)
            )
        </div>
    })
}

#[component]
async fn scope_switch(guild_id: &str, board: LeaderboardView) -> Result<impl View> {
    let segment = path_segment(guild_id);
    let server = LeaderboardView::with_scope(false).href(&segment);
    let global = LeaderboardView::with_scope(true).href(&segment);

    Ok(view! {
        <div class="segmented" role="tablist">
            <a class=(if board.global { "seg" } else { "seg active" }) href=(server)>
                "This server"
            </a>
            <a class=(if board.global { "seg active" } else { "seg" }) href=(global)>
                "Global"
            </a>
        </div>
    })
}

#[component]
async fn leaderboard(
    cx: &Cx,
    guild_id: &str,
    board: LeaderboardView,
) -> Result<impl View> {
    let page = get_leaderboard(cx, guild_id, board.global, board.page)
        .await
        .map_err(server_error_text);

    Ok(view! {
        match page {
            Err(error) => <p class="error">
                "Failed to load leaderboard: "
                (error)
            </p>,
            Ok(page) => {
                if page.entries.is_empty() {
                    <div class="empty">(board.empty_text())</div>
                } else {
                    <div class="leaderboard">
                        <div class="lb-row lb-head">
                            <span class="lb-rank">"#"</span>
                            <span class="lb-user">"Member"</span>
                            <span class="lb-num">"Level"</span>
                            <span class="lb-num">"XP"</span>
                            <span class="lb-num">"Messages"</span>
                        </div>
                        #[key(entry.rank)]
                        for entry in &page.entries {
                            entry_row(entry: entry)
                        }
                    </div>
                }
                if board.shows_pager(page.has_next) {
                    pager(guild_id: guild_id, board: board, has_next: page.has_next)
                }
            }
        }
    })
}

#[component]
async fn entry_row(entry: &LeaderboardEntry) -> Result<impl View> {
    Ok(view! {
        <div class="lb-row">
            <span class="lb-rank">(entry.rank)</span>
            <span class="lb-user">
                match &entry.avatar {
                    Some(url) => <img class="lb-avatar" src=(url.as_str()) alt="">,
                    None => <span class="lb-avatar placeholder"></span>,
                }
                <span class="lb-name">(entry.name.as_str())</span>
            </span>
            <span class="lb-num">(entry.level)</span>
            <span class="lb-num">(entry.xp)</span>
            <span class="lb-num">(entry.message_count)</span>
        </div>
    })
}

#[component]
async fn pager(
    guild_id: &str,
    board: LeaderboardView,
    has_next: bool,
) -> Result<impl View> {
    let segment = path_segment(guild_id);
    let previous = board.previous().map(|view| view.href(&segment));
    let next = board.next(has_next).map(|view| view.href(&segment));

    Ok(view! {
        <div class="pager">
            match previous {
                Some(href) => <a class="btn btn-secondary" href=(href)>"Previous"</a>,
                None => <button type="button" class="btn btn-secondary" disabled="">
                    "Previous"
                </button>,
            }
            <span class="pager-page">
                "Page "
                (board.page)
            </span>
            match next {
                Some(href) => <a class="btn btn-secondary" href=(href)>"Next"</a>,
                None => <button type="button" class="btn btn-secondary" disabled="">
                    "Next"
                </button>,
            }
        </div>
    })
}
