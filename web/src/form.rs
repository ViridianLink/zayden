/// Why a submitted form did not have the shape its save expects.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    #[error("unknown field `{0}`")]
    Unknown(String),
    #[error("duplicate field `{0}`")]
    Duplicate(String),
    #[error("missing field `{0}`")]
    Missing(&'static str),
}

/// Folds submitted `(name, value)` pairs into one value per expected name, in
/// the order given. Every name must appear exactly once and no other name may
/// appear.
pub fn fold<const N: usize>(
    pairs: Vec<(String, String)>,
    names: [&'static str; N],
) -> Result<[String; N], FieldError> {
    let mut values: [Option<String>; N] = [const { None }; N];

    for (key, value) in pairs {
        let slot = names
            .iter()
            .position(|name| *name == key)
            .and_then(|i| values.get_mut(i));

        match slot {
            None => return Err(FieldError::Unknown(key)),
            Some(Some(_)) => return Err(FieldError::Duplicate(key)),
            Some(slot) => *slot = Some(value),
        }
    }

    if let Some((name, _)) =
        names.iter().zip(&values).find(|(_, value)| value.is_none())
    {
        return Err(FieldError::Missing(name));
    }

    Ok(values.map(Option::unwrap_or_default))
}
