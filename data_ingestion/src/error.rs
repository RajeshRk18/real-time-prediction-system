use config;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataIngestionError {
    #[error("HTTP request error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("JSON deserialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Protobuf decoding error: {0}")]
    ProstDecodeError(#[from] prost::DecodeError),

    #[error("Config not found: {0}")]
    ConfigValueNotFoundError(#[from] config::ConfigError),

    #[error("Failed to send to receiver: {0}")]
    MpscError(#[from] tokio::sync::mpsc::error::SendError<String>),

    #[error("Websocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
}
