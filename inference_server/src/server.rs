use anyhow::Result;
use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::{
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::mpsc;
use crate::misc::Output;

/// Shared application state to store the latest output.
#[derive(Clone)]
struct AppState {
    latest_output: Arc<RwLock<Option<Output>>>,
}

///  GET /output returns the latest inference output.
async fn get_latest_output(State(state): State<AppState>) -> Json<Option<Output>> {
    let output = state.latest_output.read().clone();
    Json(output)
}

pub struct Server {
    state: AppState,
    output_rx: mpsc::Receiver<Output>,
}

impl Server {
    pub fn init(output_rx: mpsc::Receiver<Output>) -> Self {
        let state = AppState {
            latest_output: Arc::new(RwLock::new(None)),
        };

        Self {
            state,
            output_rx,
        }        
    }

    pub async fn receive_output(&mut self) -> Result<()> {
        while let Some(output) = self.output_rx.recv().await {
            println!("Received inference output: {:?}", output);
            let mut lock = self.latest_output.write();
            *lock = Some(output);
        }
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let state = AppState {
            latest_output: self.latest_output.clone(),
        };

        let app = Router::new()
            .route("/output", get(get_latest_output))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        println!("HTTP server running on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a channel for inference outputs.
    let (output_tx, mut output_rx) = mpsc::channel::<Output>(100);

    // Create shared state for storing the latest output.
    let state = AppState {
        latest_output: Arc::new(RwLock::new(None)),
    };
    let shared_state = state.clone();

    // Spawn a task that continuously receives outputs from the inference engine
    // and updates the shared state.
    tokio::spawn(async move {
        while let Some(output) = output_rx.recv().await {
            println!("Received inference output: {:?}", output);
            let mut lock = shared_state.latest_output.write();
            *lock = Some(output);
        }
    });

    // Here you would run your inference engine, passing `output_tx` to it.
    // For example:
    // tokio::spawn(async move {
    //     let mut engine = InferenceEngine::new(..., output_tx).await.unwrap();
    //     engine.run_inference().await.unwrap();
    // });

    // Build the axum application with a route to get the latest output.
    let app = Router::new()
        .route("/output", get(get_latest_output))
        .with_state(state);

    // Bind and serve the HTTP server.
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/*
/*
# Stage 1: Build
FROM rust:1.66 as builder
WORKDIR /app

# Cache dependencies. Copy Cargo.toml/Cargo.lock and a dummy src/main.rs.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy the actual source code and rebuild
COPY . .
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
# Copy the binary. Replace `data_ingestion` with your actual binary name.
COPY --from=builder /app/target/release/data_ingestion /usr/local/bin/data_ingestion
EXPOSE 8081
CMD ["data_ingestion"]


# Stage 1: Build
FROM rust:1.66 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
COPY . .
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/feature_engineering /usr/local/bin/feature_engineering
EXPOSE 8082
CMD ["feature_engineering"]

# Stage 1: Build
FROM rust:1.66 as builder
WORKDIR /app
# Ensure that any needed ONNX runtime libraries are linked.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
COPY . .
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:buster-slim
# Install dependencies (if your ONNX runtime crate requires additional libraries)
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates libprotobuf-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/inference /usr/local/bin/inference
# Also copy the ONNX model file into the container:
COPY logistic_regression_sentiment.onnx /usr/local/bin/logistic_regression_sentiment.onnx
EXPOSE 8083
CMD ["inference"]



version: "3.8"
services:
  data_ingestion:
    build:
      context: ./data_ingestion
    container_name: data_ingestion
    ports:
      - "8081:8081"
    restart: always

  feature_engineering:
    build:
      context: ./feature_engineering
    container_name: feature_engineering
    ports:
      - "8082:8082"
    restart: always
    depends_on:
      - data_ingestion

  inference:
    build:
      context: ./inference
    container_name: inference
    ports:
      - "8083:8083"
    restart: always
    depends_on:
      - feature_engineering

  api_server:
    build:
      context: ./api_server
    container_name: api_server
    ports:
      - "8080:8080"
    restart: always
    depends_on:
      - inference

  trading_execution:
    build:
      context: ./trading_execution
    container_name: trading_execution
    ports:
      - "8084:8084"
    restart: always
    depends_on:
      - api_server


# docker-compose.yml
version: "3.8"
services:
  kafka:
    image: bitnami/kafka:latest
    environment:
      KAFKA_CFG_ZOOKEEPER_CONNECT: "zookeeper:2181"
      ALLOW_PLAINTEXT_LISTENER: "yes"
    ports:
      - "9092:9092"

  zookeeper:
    image: bitnami/zookeeper:latest
    environment:
      ALLOW_ANONYMOUS_LOGIN: "yes"

  redis:
    image: redis:latest
    restart: always
    ports:
      - "6379:6379"

  postgres:
    image: postgres:latest
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
      POSTGRES_DB: trading_db
    ports:
      - "5432:5432"

  data_ingestion:
    build: ./data_ingestion
    depends_on:
      - kafka

  preprocessing:
    build: ./preprocessing
    depends_on:
      - kafka
      - redis

  inference_service:
    build: ./inference_service
    depends_on:
      - preprocessing

  decision_engine:
    build: ./decision_engine
    depends_on:
      - inference_service

  execution_service:
    build: ./execution_service
    depends_on:
      - decision_engine


*/

*/