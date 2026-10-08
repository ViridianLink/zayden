use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::{page, path_param};
use topcoat::view::{View, ViewExt, component, suspense, view};

use super::action::path_segment;
use super::fields::{load_error, load_problem};
use super::header::page_header;
use super::state::PageState;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::empty_state::empty_state;
use crate::components::flash::flash;
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::engagement::levels::get_leaderboard;
use crate::engagement::{LeaderboardEntry, LeaderboardView};
use crate::shell::GuildId;

const COLUMNS: &[&str] = &["Rank", "Member", "Level", "XP", "Messages"];

#[page("/guild/{guild_id}/levels")]
pub(super) async fn levels(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let board = LeaderboardView::from_request(cx);
    let state = PageState::load(cx);

    Ok(view! { levels_page(guild_id: guild_id, board: board, state: &state) })
}

#[component]
async fn levels_page(
    guild_id: &str,
    board: LeaderboardView,
    state: &PageState,
) -> Result<impl View> {
    Ok(view! {
        <div class="page">
            page_header(
                guild_id: guild_id,
                title: "Levels",
                "Message-XP rankings. Switch between this server and the global board."
            )
            flash(notice: state.top_notice())
            scope_switch(guild_id: guild_id, board: board)
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        shape_skeleton(shape: SkeletonShape::Row, count: 10)
                    </div>
                },
                leaderboard(guild_id: guild_id, board: board)
            )
        </div>
    }
    .boxed())
}

/// The two boards as links, the current one marked for assistive
/// technology.
#[component]
async fn scope_switch(guild_id: &str, board: LeaderboardView) -> Result<impl View> {
    let segment = path_segment(guild_id);
    let server = LeaderboardView::with_scope(false).href(&segment);
    let global = LeaderboardView::with_scope(true).href(&segment);
    let current = move |global: bool| (board.global == global).then_some("page");

    Ok(view! {
        <nav class="segmented" aria-label="Leaderboard">
            <a
                class=(if board.global { "seg" } else { "seg active" })
                href=(server)
                aria-current=(current(false))
            >
                "This server"
            </a>
            <a
                class=(if board.global { "seg active" } else { "seg" })
                href=(global)
                aria-current=(current(true))
            >
                "Global"
            </a>
        </nav>
    })
}

fn caption(board: LeaderboardView) -> String {
    let scope = if board.global { "Global" } else { "This server" };
    format!("{scope} leaderboard, page {}", board.page)
}

#[component]
async fn leaderboard(
    cx: &Cx,
    guild_id: &str,
    board: LeaderboardView,
) -> Result<impl View> {
    let page = get_leaderboard(cx, guild_id, board.global, board.page).await;
    let segment = path_segment(guild_id);
    let first = LeaderboardView { page: 1, ..board }.href(&segment);
    let caption = caption(board);

    Ok(view! {
        match page {
            Err(error) => {
                let (message, actions) =
                    load_problem(guild_id, &board.href(&segment), &error);
                load_error(
                    title: "Couldn't load the leaderboard",
                    message: &message,
                    actions: &actions
                )
            }
            Ok(page) => {
                if page.entries.is_empty() {
                    if board.page > 1 {
                        empty_state(
                            title: "No one on this page",
                            text: board.empty_text(),
                            action: Some("Back to page 1"),
                            href: Some(&first)
                        )
                    } else {
                        empty_state(
                            title: "No one on the board yet",
                            text: board.empty_text()
                        )
                    }
                } else {
                    data_table(
                        caption: &caption,
                        columns: COLUMNS,
                        #[key(entry.rank)]
                        for entry in &page.entries {
                            entry_row(entry: entry)
                        }
                    )
                }
                if board.shows_pager(page.has_next) {
                    pager(guild_id: guild_id, board: board, has_next: page.has_next)
                }
            }
        }
    }
    .boxed())
}

#[component]
async fn entry_row(entry: &LeaderboardEntry) -> Result<impl View> {
    Ok(view! {
        data_row(
            data_cell(label: "Rank", numeric: true, (entry.rank))
            data_cell(
                label: "Member",
                header: true,
                match &entry.avatar {
                    Some(url) => <img class="lb-avatar" src=(url.as_str()) alt="">,
                    None => <span class="lb-avatar placeholder"></span>,
                }
                <span class="lb-name">(entry.name.as_str())</span>
            )
            data_cell(label: "Level", numeric: true, (entry.level))
            data_cell(label: "XP", numeric: true, (entry.xp))
            data_cell(label: "Messages", numeric: true, (entry.message_count))
        )
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
        <nav class="pager" aria-label="Leaderboard pages">
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
        </nav>
    })
}
