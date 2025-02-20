use anyhow::Result;
use axum::response::IntoResponse;
use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use log::info;
use crate::misc::Output;

/// Shared application state to store the latest output.
#[derive(Clone)]
struct AppState {
    latest_output: Arc<RwLock<Option<Output>>>,
}

///  GET /output returns the latest inference output.
async fn get_latest_output(State(state): State<AppState>) -> impl IntoResponse {
    let output = state.latest_output.read();
    Json(output.clone())
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
            let mut lock = self.state.latest_output.write();
            *lock = Some(output);
        }
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let state = AppState {
            latest_output: self.state.latest_output.clone(),
        };

        let app = Router::new()
            .route("/output", get(get_latest_output))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("HTTP server running on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}