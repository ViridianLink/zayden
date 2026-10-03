/// Declares the fields a dashboard form posts, as `String`s named like the
/// form inputs, with a `from_pairs` fold over the urlencoded body.
macro_rules! form_args {
    ($(#[$meta:meta])* $name:ident { $($field:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Default, PartialEq, Eq)]
        pub struct $name {
            $(pub $field: String,)+
        }

        impl $name {
            pub fn from_pairs(
                pairs: Vec<(String, String)>,
            ) -> Result<Self, $crate::guild::GuildError> {
                let [$($field),+] =
                    $crate::form::fold(pairs, [$(stringify!($field)),+])?;
                Ok(Self { $($field),+ })
            }
        }
    };
}

pub(crate) use form_args;

form_args! {
    GuildForm { guild }
}
