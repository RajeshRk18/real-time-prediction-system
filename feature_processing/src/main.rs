mod consumer;
mod processor;
mod misc;
mod feature_producer;

use crate::consumer::KafkaConsumer;
use crate::feature_producer::KafkaProducer;
use crate::processor::WindowedMarketDataStream;
use data_ingestion::logger::init_logger;
use tokio::sync::mpsc;
use tokio::signal;
use anyhow::Result;
use log::*;

struct Pipeline {
    data_consumer: KafkaConsumer,
    feat_producer: KafkaProducer,
    processor: WindowedMarketDataStream,
}

impl Pipeline {
    async fn new() -> Result<Self> {
        let (input_tx, input_rx) = mpsc::channel(1000);
        let (feature_tx, feature_rx) = mpsc::channel(1000);

        Ok(Self {
            data_consumer: KafkaConsumer::new(input_tx).await?,
            feat_producer: KafkaProducer::new("FEATURES", feature_rx).await?,
            processor: WindowedMarketDataStream::init(input_rx, feature_tx),
        })
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = signal::ctrl_c() => {
                    info!("Received shutdown signal, initiating graceful shutdown");
                    break;
                }
                // Consumer task
                res = self.data_consumer.consume() => {
                    if let Err(e) = res {
                        error!("Data consumer failed: {:?}", e);
                        break;
                    }
                }
                // Producer task
                res = self.feat_producer.receive() => {
                    if let Err(e) = res {
                        error!("Feature producer failed: {:?}", e);
                        break;
                    }
                }
                // Processor task
                res = self.processor.start_processing() => {
                    if let Err(e) = res {
                        error!("Processor failed: {:?}", e);
                        break;
                    }
                }
            }
        }

        /*// if ctrl c or any task fails, then a clean shutdown with proper cleanup of resources is done
        self.shutdown().await?;*/
        Ok(())
    }

    // THIS CAN BE DONE IF EVERY TASKS IMPLEMENT A GRACEFUL SHUTDOWN WITH PROPER CLEAN UP
    /*async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down pipeline components");

        let shutdown_timeout = tokio::time::Duration::from_secs(5);

        tokio::select! {
            res = self.data_consumer.shutdown() => {
                res.context("Failed to shutdown consumer")?;
            }
            _ = tokio::time::sleep(shutdown_timeout.clone()) => {
                error!("Consumer shutdown timed out");
            }
        }

        tokio::select! {
            res = self.feat_producer.shutdown() => {
                res.context("Failed to shutdown producer")?;
            }
            _ = tokio::time::sleep(shutdown_timeout.clone()) => {
                error!("Producer shutdown timed out");
            }
        }

        tokio::select! {
            res = self.processor.shutdown() => {
                res.context("Failed to shutdown processor")?;
            }
            _ = tokio::time::sleep(shutdown_timeout) => {
                error!("Processor shutdown timed out");
            }
        }

        info!("Pipeline shutdown complete");
        Ok(())
    }*/
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();
    let mut pipeline = Pipeline::new().await?;

    pipeline.run().await?;

    info!("Pipeline has been shut down!");

    Ok(())
}