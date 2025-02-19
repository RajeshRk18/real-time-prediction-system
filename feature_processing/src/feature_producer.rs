use polars::prelude::DataFrame;
use anyhow::Result;
use log::{debug, error, warn};
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::mpsc;
use tokio::time::Duration;
use bincode;
use log::info;

use crate::misc::Features;

pub struct KafkaProducer {
    producer: FutureProducer,
    receiver_handle: mpsc::Receiver<Features>,
    topic: String,
}

impl KafkaProducer {
    pub async fn new(
        topic: &str,
        receiver_handle: mpsc::Receiver<DataFrame>,
    ) -> Result<Self> {
        info!("🚀Configuring Kafka Producer...");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", "kafka:9092")
            .set("message.timeout.ms", "30000")
            .set("compression.type", "snappy")
            .set("queue.buffering.max.messages", "100000")
            .set("batch.num.messages", "1000")
            .set("linger.ms", "10")
            .set("enable.idempotence", "true")
            .create()?;

        info!("Configuration done✅");
        Ok(Self {
            producer,
            receiver_handle,
            topic: topic.to_string(),
        })
    }

    pub async fn receive(&mut self) -> Result<()> {
        while let Some(input_data) = self.receiver_handle.recv().await {
            debug!("Recived a record");
            self.produce(input_data).await?;
        }
        Ok(())
    }

    async fn produce(&self, data: InputData) -> Result<()> {
        let value = bincode::serialize(&data)?;
        // Create Kafka record
        let record = FutureRecord::to(&self.topic)
            .key("features")
            .payload(&value);

        // Send to Kafka
        let delivery_status = self.producer.send(record, Duration::from_secs(30)).await;
        match delivery_status {
            Ok(_) => debug!("Message delivered successfully"),
            Err((e, _)) => error!("Delivery to Kafka failed: {}", e),
        }

        Ok(())
    }
}