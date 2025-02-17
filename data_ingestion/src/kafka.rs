use crate::config::KafkaBrokerConfig;
use crate::fetcher::InputData;
use anyhow::Result;
use log::{debug, error, warn};
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::mpsc;
use tokio::time::Duration;
use bincode;
use log::info;

pub struct KafkaProducer {
    producer: FutureProducer,
    receiver_handle: mpsc::Receiver<InputData>,
    topic: String,
    config: KafkaBrokerConfig,
}

impl KafkaProducer {
    pub async fn new(
        topic: &str,
        config: KafkaBrokerConfig,
        receiver_handle: mpsc::Receiver<InputData>,
    ) -> Result<Self> {
        info!("🚀Configuring Kafka Producer...");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.broker_url.clone())
            .set("message.timeout.ms", "30000")
            .set("compression.type", "snappy")
            .set("queue.buffering.max.messages", "100000")
            .set("batch.num.messages", "1000")
            .set("linger.ms", "10")
            .set("enable.idempotence", "true")
            .create()?;

        //info!("🚀Configuring Avro Schema Registry...");
        info!("Configuration done✅");
        Ok(Self {
            producer,
            receiver_handle,
            topic: topic.to_string(),
            config,
        })
    }

    pub async fn receive(&mut self) -> Result<()> {
        while let Some(input_data) = self.receiver_handle.recv().await {
            if !validate_data(&input_data) {
                error!("Invalid Data received!");
                return Ok(());
            }
            debug!("Recived a record");
            self.produce(input_data).await?;
        }
        Ok(())
    }

    pub async fn produce(&self, data: InputData) -> Result<()> {
        let value = bincode::serialize(&data)?;
        // Create Kafka record
        let record = FutureRecord::to(&self.topic)
            .key(&self.config.instrument_key)
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

/*pub struct KafkaProducer {
    producer: FutureProducer,
    avro_encoder: AvroEncoder<'static>,
    receiver_handle: mpsc::Receiver<InputData>,
    topic: String,
    config: KafkaBrokerConfig,
}

impl KafkaProducer {
    pub async fn new(
        topic: &str,
        config: KafkaBrokerConfig,
        receiver_handle: mpsc::Receiver<InputData>,
    ) -> Result<Self> {
        info!("🚀Configuring Kafka Producer...");
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.broker_url.clone())
            .set("message.timeout.ms", "30000")
            .set("compression.type", "snappy")
            .set("queue.buffering.max.messages", "100000")
            .set("batch.num.messages", "1000")
            .set("linger.ms", "10")
            .set("enable.idempotence", "true")
            .create()?;

        info!("🚀Configuring Avro Schema Registry...");
        let sr_settings = SrSettings::new(config.schema_registry_url.clone());
        let avro_encoder = AvroEncoder::new(sr_settings);
        println!("{:?}", avro_encoder);
        info!("Configuration done✅");
        Ok(Self {
            producer,
            avro_encoder,
            receiver_handle,
            topic: topic.to_string(),
            config,
        })
    }

    pub async fn receive(&mut self) -> Result<()> {
        while let Some(input_data) = self.receiver_handle.recv().await {
            if !validate_data(&input_data) {
                error!("Invalid Data received!");
                return Ok(());
            }
            info!("Recived a record");
            self.produce(input_data).await?;
        }
        Ok(())
    }

    async fn produce(&self, data: InputData) -> Result<()> {
        // Serialize to Avro
        let subject_name_strategy =
            SubjectNameStrategy::RecordNameStrategy(self.topic.clone());

        let value = self
            .avro_encoder
            .encode_struct(data, &subject_name_strategy)
            .await?;

        // Create Kafka record
        let record = FutureRecord::to(&self.topic)
            .key(&self.config.instrument_key)
            .payload(&value);

        // Send to Kafka
        let delivery_status = self.producer.send(record, Duration::from_secs(30)).await;
        match delivery_status {
            Ok(_) => debug!("Message delivered successfully"),
            Err((e, _)) => error!("Delivery to Kafka failed: {}", e),
        }

        Ok(())
    }
//docker compose:

  avro-schema-registry:
    image: confluentinc/cp-schema-registry:7.8.0
    depends_on:
      - kafka
    ports:
      - "9094:9094"
    container_name: avro-schema-registry
  
    environment:
      SCHEMA_REGISTRY_HOST_NAME: avro-schema-registry
      SCHEMA_REGISTRY_LISTENERS: http://0.0.0.0:9094
      SCHEMA_REGISTRY_KAFKASTORE_BOOTSTRAP_SERVERS: PLAINTEXT://kafka:9092
}*/

pub fn validate_data(data: &InputData) -> bool {
    let mut is_valid = true;
    if data.vol < 0 {
        warn!("Invalid Volume data");
        is_valid = false;
    }

    if data.open < 0 as f64 {
        warn!("Open cannot be negative");
        is_valid = false;
    }

    if data.close < 0 as f64 {
        warn!("Close cannot be negative");
        is_valid = false;
    }

    if data.high < 0 as f64 {
        warn!("High cannot be negative");
        is_valid = false;
    }

    if data.low < 0 as f64 {
        warn!("Low cannot be negative");
        is_valid = false;
    }

    is_valid
}

