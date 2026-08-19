pub mod adopt;
pub mod marry;

use serenity::all::UserId;

pub const MARRY_ACCEPT: &str = "marry_accept";
pub const MARRY_DECLINE: &str = "marry_decline";
pub const ADOPT_ACCEPT: &str = "adopt_accept";
pub const ADOPT_DECLINE: &str = "adopt_decline";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptAuth {
    Allowed,
    SelfAccept,
    Unauthorised,
}

#[must_use]
pub const fn accept_auth(
    author: UserId,
    responder: UserId,
    is_mentioned: bool,
) -> AcceptAuth {
    if responder.get() == author.get() {
        AcceptAuth::SelfAccept
    } else if is_mentioned {
        AcceptAuth::Allowed
    } else {
        AcceptAuth::Unauthorised
    }
}
