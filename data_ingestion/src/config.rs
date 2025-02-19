use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WebSocketConfig {
    pub api_key: String,
    pub api_secret_key: String,
    pub redirect_uri: String,
    pub user_auth_url: String,
    pub md_auth_url: String,
    pub instrument_key: String,
}

impl WebSocketConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let cfg = Config::builder()
            .add_source(File::new("../websocket.toml", FileFormat::Toml))
            .build()?;

        cfg.try_deserialize()
    }
}

#[derive(Deserialize)]
pub struct KafkaBrokerConfig {
    pub instrument_key: String,
    pub broker_url: String,
    pub message_timeout_ms: String,
    pub compression_type: String,
    pub queue_buffer_max_msg: String,
    pub batch_num_messages: String,
    pub linger_ms: String,
    pub enable_idempotence: String,
}

impl KafkaBrokerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let cfg = Config::builder()
            .add_source(File::new("../kafka.toml", FileFormat::Toml))
            .build()?;
        cfg.try_deserialize()
    }
}