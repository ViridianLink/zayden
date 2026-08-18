pub mod adopt;
pub mod marry;

use serenity::all::UserId;

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
