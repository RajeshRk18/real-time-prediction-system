use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InputData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vol: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Features {
    pub price_change: f64,
    pub price_momentum: f64,
    pub price_volatility: f64,
    pub volume_ratio: f64,
    pub volume_momentum: f64,
    pub price_acceleration: f64,
}