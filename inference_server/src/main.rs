mod consumer;
mod engine;
mod misc;
mod server;

use crate::consumer::KafkaConsumer;
use crate::engine::InferenceEngine;
use crate::server::Server;
use data_ingestion::logger::init_logger;
use tokio::sync::mpsc;
use tokio::signal;
use anyhow::Result;
use log::*;

struct Pipeline {
    feat_consumer: KafkaConsumer,
    inference: InferenceEngine,
    server: Server,
}

impl Pipeline {
    async fn new() -> Result<Self> {
        let (feature_tx, feature_rx) = mpsc::channel(1000);
        let (output_tx, output_rx) = mpsc::channel(1000);

        Ok(Self {
            feat_consumer: KafkaConsumer::new(feature_tx).await?,
            inference: InferenceEngine::new(feature_rx, output_tx)?,
            server: Server::init(output_rx),
        })
    }

    async fn run(&mut self) -> Result<()> {
        self.server.run().await?;

        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = signal::ctrl_c() => {
                    info!("Received shutdown signal, initiating graceful shutdown");
                    break;
                }
                // Consumer task
                res = self.feat_consumer.consume() => {
                    if let Err(e) = res {
                        error!("Feature consumer failed: {:?}", e);
                        break;
                    }
                }
                // Inference
                res = self.inference.run_inference() => {
                    if let Err(e) = res {
                        error!("Inference engine failed: {:?}", e);
                        break;
                    }
                }
                // Output receive handler
                res = self.server.receive_output() => {
                    if let Err(_) = res {
                        break;
                    }                    
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()>{
    init_logger();

    let mut pipeline = Pipeline::new().await?;

    pipeline.run().await?;

    info!("Pipeline has been shut down gracefully");

    Ok(())
}
