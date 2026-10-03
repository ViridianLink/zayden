use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::request::{content_type, uri};

use super::error::{FieldError, RequestRejection};

const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

fn pick<const N: usize>(
    pairs: &[(String, String)],
    keys: [&'static str; N],
) -> Result<[Option<String>; N], FieldError> {
    let mut found: [Option<String>; N] = std::array::from_fn(|_| None);

    for (name, value) in pairs {
        let Some(index) = keys.iter().position(|key| key == name) else {
            continue;
        };
        let (Some(slot), Some(key)) = (found.get_mut(index), keys.get(index)) else {
            continue;
        };
        if slot.is_some() {
            return Err(FieldError::Duplicate(key));
        }
        *slot = Some(value.clone());
    }

    Ok(found)
}

fn parse_pairs(bytes: &[u8]) -> Vec<(String, String)> {
    Form::<Vec<(String, String)>>::from_bytes(bytes)
        .map_or_default(|Form(pairs)| pairs)
}

pub(super) fn query<const N: usize>(
    cx: &Cx,
    keys: [&'static str; N],
) -> Result<[Option<String>; N], RequestRejection> {
    let query = uri(cx).query().unwrap_or_default();
    pick(&parse_pairs(query.as_bytes()), keys).map_err(RequestRejection::Query)
}

pub(super) fn form<const N: usize>(
    cx: &Cx,
    body: &[u8],
    keys: [&'static str; N],
) -> Result<[Option<String>; N], RequestRejection> {
    if !content_type(cx).is_some_and(|value| value.starts_with(FORM_CONTENT_TYPE)) {
        return Err(RequestRejection::FormContentType);
    }

    pick(&parse_pairs(body), keys).map_err(RequestRejection::FormBody)
}

pub(super) fn required_query(
    value: Option<String>,
    key: &'static str,
) -> Result<String, RequestRejection> {
    value.ok_or(RequestRejection::Query(FieldError::Missing(key)))
}

pub(super) fn required_form(
    value: Option<String>,
    key: &'static str,
) -> Result<String, RequestRejection> {
    value.ok_or(RequestRejection::FormBody(FieldError::Missing(key)))
}

pub(super) fn integer(
    value: Option<&str>,
    key: &'static str,
) -> Result<Option<i64>, RequestRejection> {
    value
        .map(|raw| {
            raw.parse::<i64>().map_err(|e| {
                RequestRejection::Query(FieldError::Invalid {
                    key,
                    reason: e.to_string(),
                })
            })
        })
        .transpose()
}
