mod config;
mod error;
mod fetcher;
mod kafka;
mod logger;

use tokio;
use tokio::sync::mpsc;
use anyhow::Result;
use crate::fetcher::{StockDataStream, InputData};
use crate::logger::init_logger;
use crate::kafka::KafkaProducer;
use crate::config::{KafkaBrokerConfig, WebSocketConfig};

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    // Webosocket
    let (ws_sender, mut ws_receiver) = mpsc::channel::<InputData>(100);
    let ws_cfg = WebSocketConfig::from_env()?;
    let mut stream = StockDataStream::new(ws_sender, ws_cfg);
    stream.authorize().await?;

    // Kafka 
    let config = KafkaBrokerConfig::from_env()?;
    let (kafka_sender, kafka_receiver) = mpsc::channel::<InputData>(1000);
    // Create producer
    let mut producer = KafkaProducer::new("market-data", config, kafka_receiver).await?;
    producer.receive().await?;

    log::info!("Starting market data producer..");

    // Graceful shutdown handler
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        log::info!("Shutting down...");
    };

    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = async {
                while let Some(data) = ws_receiver.recv().await {
                    if let Err(e) = kafka_sender.send(data).await {
                        log::error!("Failed to send data to Kafka channel: {}", e);
                    }
                }
            } => {},
            _ = &mut shutdown => break,
        }
    }

    Ok(())
}

