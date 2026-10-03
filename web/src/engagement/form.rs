macro_rules! form_args {
    ($(#[$meta:meta])* $name:ident { $($field:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name {
            $(pub $field: String,)+
        }

        impl $name {
            pub fn from_pairs(
                pairs: Vec<(String, String)>,
            ) -> Result<Self, $crate::engagement::EngagementError> {
                let [$($field),+] =
                    $crate::form::fold(pairs, [$(stringify!($field)),+])?;
                Ok(Self { $($field),+ })
            }

            pub fn ensure_path_guild(
                &self,
                path_guild: &str,
            ) -> Result<(), $crate::engagement::EngagementError> {
                if self.guild == path_guild {
                    Ok(())
                } else {
                    Err($crate::engagement::EngagementError::GuildMismatch)
                }
            }
        }
    };
}

pub(crate) use form_args;
