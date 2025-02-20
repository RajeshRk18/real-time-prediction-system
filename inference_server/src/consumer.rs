use feature_processing::misc::Features;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use rdkafka::{ClientConfig, message::Message};
use rdkafka::consumer::{StreamConsumer, Consumer};
use bincode;
use anyhow::Result;
use log::{info, error, debug};
use polars::prelude::DataFrame;

pub struct KafkaConsumer {
    consumer: StreamConsumer,
    processor_handler: mpsc::Sender<Features>,
}

impl KafkaConsumer {
    pub async fn new(processor_handler: mpsc::Sender<Features>) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "feature-engineering-group")
        .set("bootstrap.servers", "kafka:9092")
        .set("enable.auto.commit", "true")
        .set("auto.offset.reset", "earliest")
        .create()?;

        consumer
            .subscribe(&["features"])?;

        info!("🚀Feature Processor Service is now running...");

        Ok(Self {
            consumer,
            processor_handler,
        })
    }

    pub async fn consume(&mut self) -> Result<()> {
        let mut message_stream = self.consumer.stream();

        while let Some(message_result) = message_stream.next().await {
            match message_result {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let data: Features = bincode::deserialize(payload)?;
                        // Perform feature processing 
                        self.processor_handler.send(data).await?;
                    
                    } else {
                        debug!("Received message with empty payload");
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                }
            }
        }
    
        Ok(())
    }
}
