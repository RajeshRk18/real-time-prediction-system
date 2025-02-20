use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Output {
    pub buy_prob: f64,
    pub sell_prob: f64,
}