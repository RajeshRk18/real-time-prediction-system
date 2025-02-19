use anyhow::Result;
use polars::prelude::*;
use parking_lot::RwLock;
use polars::series::ops::NullBehavior;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::collections::VecDeque;
use crate::misc::Features;
use data_ingestion::fetcher::InputData;

const WINDOW_SIZE: usize = 10;

pub struct WindowedMarketDataStream {
    data: VecDeque<InputData>,
    receiver: mpsc::Receiver<InputData>,
    producer_handler: mpsc::Sender<Features>,
}

impl WindowedMarketDataStream {
    pub fn init(receiver: mpsc::Receiver<InputData>, producer_handler: mpsc::Sender<Features>) -> Self {
        let data = VecDeque::new();
        Self {
            data,
            receiver,
            producer_handler,
        }
    }

    pub async fn start_processing(&mut self) -> Result<()> {
        while let Some(received) = self.receiver.recv().await {
            self.insert(received);
            let data_size = self.data.len();

            if data_size < WINDOW_SIZE {
                continue;
            }

            let df = self.to_dataframe()?;

            let windowed_features = extract_features(df)?;

            self.producer_handler.send(windowed_features).await?;
        }
        Ok(())
    }


    fn to_dataframe(&self) -> Result<DataFrame> {
        let close: Vec<f64> = self.data.iter().map(|d| d.close).collect();
        let vol: Vec<i64> = self.data.iter().map(|d| d.vol).collect();

        let s1 = Series::new("close".into(), close);
        let s2 = Series::new("vol".into(), vol);
        let df = DataFrame::new(vec![s1, s2])?;
        Ok(df)
    }

    fn insert(&mut self, data: InputData) {
        if self.data.len() == WINDOW_SIZE {
            self.data.pop_front();
        }
        self.data.push_back(data);
    }
}

/// Extract features from a DataFrame
fn extract_features(df: DataFrame) -> Result<Features> {
    let lf = df.lazy();
    
    let df = lf
        .with_columns([
            col("close").shift(Expr::Nth(1)),
            // Price change (1-period percentage change)
            col("close").pct_change(Expr::Nth(1)).alias("price_change"),
            
            // Price momentum (5-period mean of price changes)
            col("close").pct_change(Expr::Nth(1))
                .rolling_mean(RollingOptions {
                    window_size: Duration::parse("5i"),
                    min_periods: 1,
                    ..Default::default()
                })
                .alias("price_momentum"),
            
            // Price volatility (10-period std of price changes)
            col("close").pct_change(Expr::Nth(1))
                .rolling_std(RollingOptions {
                    window_size: Duration::parse("10i"),
                    min_periods: 1,
                    ..Default::default()
                })
                .alias("price_volatility"),
            
            // Volume momentum (5-period mean of volume changes)
            col("volume").pct_change(Expr::Nth(1))
                .rolling_mean(RollingOptions {
                    window_size: Duration::parse("10i"),
                    min_periods: 1,
                    ..Default::default()
                })
                .alias("volume_momentum"),
            
            // Volume ratio (current volume vs 10-period average)
            (col("volume") / 
             col("volume").rolling_mean(RollingOptions {
                 window_size: Duration::parse("10i"),
                 min_periods: 1,
                 ..Default::default()
             }))
            .alias("volume_ratio"),
        ])
        // Price acceleration (difference of price changes)
        .with_columns([
            col("price_change").diff(1, NullBehavior::Ignore).alias("price_acceleration")
        ])
        // Drop null values created by window operations
        .drop_nulls(None)
        .collect()?;

    let price_change = df.column("price_change")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);

    let price_momentum = df.column("price_momentum")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);
    let price_volatility = df.column("price_volatility")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);
    let volume_momentum = df.column("volume_momentum")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);
    let volume_ratio = df.column("volume_ratio")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);
    let price_acceleration = df.column("price_acceleration")?.tail(Some(1)).f64()?.get(0).unwrap_or(0.0);

    let features = Features {
        price_change,
        price_momentum,
        price_volatility,
        volume_ratio,
        volume_momentum,
        price_acceleration,
    };

    Ok(features)
}


    /*let close = df.column("close")?;
    let volume = df.column("volume")?;

    // price_change: percentage change of close
    let price_change_s = (*close).pct_change(1).alias("price_change");

    // price_momentum: 5-period rolling mean of price_change
    let price_momentum = price_change_s.rolling_mean(
        RollingOptionsImpl {
            window_size: Duration::parse("5i"),
            min_periods: 5, // require full data
            ..Default::default()
        }
    )?;

    // price_volatility: 10-period rolling std dev of price_change
    let price_volatility = price_change_s.rolling_std(
        RollingOptionsImpl {
            window_size: Duration::parse("10i"),
            min_periods: 10,
            ..Default::default()
        }
    )?;

    // volume_momentum: percentage change of volume, then 5-period rolling mean
    let volume_change = pct_change(volume)?;
    let volume_momentum = volume_change.rolling_mean(
        RollingOptionsImpl {
            window_size: Duration::parse("10i"),
            min_periods: 5,
            ..Default::default()
        }
    )?;
    
    // volume_ratio: volume divided by 10-period rolling mean of volume
    let volume_rolling_mean = volume.rolling_mean(
        RollingOptionsImpl {
            window_size: Duration::parse("10i"),
            min_periods: 10,
            ..Default::default()
        }
    )?;
    let volume_ratio = volume / &volume_rolling_mean;
    // price_acceleration: first difference of price_change
    let price_acceleration = diff(&price_change_s, 1, NullBehavior::Ignore)?;*/