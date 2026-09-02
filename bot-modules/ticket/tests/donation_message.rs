use ticket::UserId;
use ticket::donation::format_message;

#[test]
fn each_helper_gets_their_own_line() {
    let helpers = vec![
        (UserId::new(1), String::from("https://ko-fi.com/a")),
        (UserId::new(2), String::from("https://ko-fi.com/b")),
    ];

    let message = format_message(&helpers);

    assert_eq!(
        message,
        "If this helped, consider supporting the people who did:\n\
         <@1>: https://ko-fi.com/a\n\
         <@2>: https://ko-fi.com/b"
    );
}

#[test]
fn a_single_helper_still_starts_on_its_own_line() {
    let helpers = vec![(UserId::new(7), String::from("https://ko-fi.com/solo"))];

    let message = format_message(&helpers);

    assert_eq!(
        message,
        "If this helped, consider supporting the people who did:\n\
         <@7>: https://ko-fi.com/solo"
    );
}

#[test]
fn no_helpers_leaves_just_the_lead() {
    assert_eq!(
        format_message(&[]),
        "If this helped, consider supporting the people who did:"
    );
}
