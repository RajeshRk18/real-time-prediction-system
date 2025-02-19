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
    let (output_tx, mut output_rx) = mpsc::channel::<Output>(100);

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