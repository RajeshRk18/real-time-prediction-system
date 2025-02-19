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
            data_consumer: KafkaConsumer::new(input_tx)?,
            feat_producer: InferenceEngine::new("FEATURES", feature_rx)?,
            server: Server::init(output_rx)?,
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
                    if let Err(e) = res {
                        break;
                    }                    
                }
                // API Server
                res = self.serve.run() => {
                    if let Err(e) = res {
                        error!("Processor failed: {:?}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    init_logger();

    let mut pipeline = Pipeline::new()?;

    pipeline.run().await?;

    info!("Pipeline has been shut down gracefully");

    Ok(())
}
