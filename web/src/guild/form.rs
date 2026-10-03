use super::error::GuildError;

/// Folds submitted `(name, value)` pairs into one value per expected name, in
/// the order given. Every name must appear exactly once and no other name may
/// appear.
pub(crate) fn fold<const N: usize>(
    pairs: Vec<(String, String)>,
    names: [&'static str; N],
) -> Result<[String; N], GuildError> {
    let mut values: [Option<String>; N] = [const { None }; N];

    for (key, value) in pairs {
        let slot = names
            .iter()
            .position(|name| *name == key)
            .and_then(|i| values.get_mut(i));

        match slot {
            None => return Err(GuildError::UnknownField(key)),
            Some(Some(_)) => return Err(GuildError::DuplicateField(key)),
            Some(slot) => *slot = Some(value),
        }
    }

    if let Some((name, _)) =
        names.iter().zip(&values).find(|(_, value)| value.is_none())
    {
        return Err(GuildError::MissingField(name));
    }

    Ok(values.map(Option::unwrap_or_default))
}

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
            /// Reads the form from its submitted pairs, rejecting unknown,
            /// repeated and missing fields.
            pub fn from_pairs(
                pairs: Vec<(String, String)>,
            ) -> Result<Self, $crate::guild::GuildError> {
                let [$($field),+] =
                    $crate::guild::form::fold(pairs, [$(stringify!($field)),+])?;
                Ok(Self { $($field),+ })
            }
        }
    };
}

pub(crate) use form_args;

form_args! {
    GuildForm { guild }
}
