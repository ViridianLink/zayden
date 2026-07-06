mod implementations;
mod registry;

pub use registry::get_effect;

pub trait GamblingEffect: Send + Sync {
    fn id(&self) -> &'static str;

    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn on_win(&self, _bet: i64, _base_payout: i64) -> i64 {
        0
    }

    fn on_loss(&self, _bet: i64, _base_payout: i64) -> i64 {
        0
    }
}
