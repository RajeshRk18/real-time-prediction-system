use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    pub price_change: f64,
    pub price_momentum: f64,
    pub price_volatility: f64,
    pub volume_ratio: f64,
    pub volume_momentum: f64,
    pub price_acceleration: f64,
}