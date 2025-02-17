use crate::misc::InputData;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::sleep;
use rdkafka::{ClientConfig, message::Message};
use rdkafka::consumer::{StreamConsumer, Consumer};
use bincode;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use log::{info, error, debug};
use std::str;
use std::time::Duration;

pub struct KafkaConsumer {
    consumer: StreamConsumer,
    handler: mpsc::Receiver<InputData>,
    processor_handler: mpsc::Sender<InputData>,
}

impl KafkaConsumer {
    pub async fn new(handler: mpsc::Receiver<InputData>, processor_handler: mpsc::Sender<InputData>) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "feature-engineering-group")
        .set("bootstrap.servers", "localhost:9092")
        // Automatically commit offsets (for simplicity)
        .set("enable.auto.commit", "true")
        // Start from the earliest available message
        .set("auto.offset.reset", "earliest")
        .create()?;

        consumer
            .subscribe(&["market-data"])?;

        info!("🚀Feature Processor Service is now running...");

        Ok(Self {
            consumer,
            handler,
            processor_handler,
        })
    }

    pub async fn consume(&mut self) -> Result<()> {
        let mut message_stream = self.consumer.stream();

        while let Some(message_result) = message_stream.next().await {
            match message_result {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let data: InputData = bincode::deserialize(payload)?;
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