use tokio::sync::mpsc;
use polars::prelude::*;
use ndarray::Array2;
use onnxruntime::{environment::Environment, GraphOptimizationLevel};
use feature_processing::misc::Features;
use crate::misc::Output;

const NUM_FEATURES: usize = 6;

pub struct InferenceEngine {
    inference: Environment,
    feature_rx: mpsc::Receiver<DataFrame>,
    output_tx: mpsc::Sender<Output>,
}

impl InferenceEngine {
    pub fn new(feature_rx: mpsc::Receiver<DataFrame>, output_tx: mpsc::Sender<Output>) -> Self {

        let environment = Environment::builder()
        .with_name("market_regime_inference")
        .build()?;

        let session = environment
            .new_session_builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_number_threads(2)?
            .with_model_from_file("../model/MarketRegimeModel.onnx")?;

        Self {
            inference: environment,
            feature_rx,
            output_tx,
        }
    }

    pub async fn run_inference(&mut self) -> Result<()> {
        while let Some(features) = self.feature_rx.recv().await {
            let input_data = vec![features.price_change, features.price_momentum, features.price_volatility, features.volume_change, features.volume_momentum, features.price_acceleration];
            let input_array = Array2::from_shape_vec((1, NUM_FEATURES), input_data)?;
            let outputs = session.run(vec![input_array.into_dyn()])?; 

            self.output_tx.send(outputs).await?;
        }
        Ok(())
    }
}