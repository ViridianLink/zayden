mod faction;
mod map;
mod runner;
mod weapon;

pub use faction::marathondb_contracts_to_factions;
pub use map::marathondb_map_to_model;
pub use runner::marathondb_runner_to_model;
use serde_json::Value;
pub use weapon::marathondb_weapon_to_model;

fn current_or_last_stats(stats: &Value) -> Option<&serde_json::Map<String, Value>> {
    let stats = stats.as_array()?;
    stats
        .iter()
        .find(|s| s.get("is_current").and_then(Value::as_bool) == Some(true))
        .or_else(|| stats.last())
        .and_then(Value::as_object)
}

fn json_scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}
