use tokio::sync::mpsc;
use onnxruntime::{environment::Environment, GraphOptimizationLevel};
use onnxruntime::ndarray::Array2;
use feature_processing::misc::Features;
use anyhow::Result;
use crate::misc::Output;

const NUM_FEATURES: usize = 6;

pub struct InferenceEngine {
    environment: Environment,
    feature_rx: mpsc::Receiver<Features>,
    output_tx: mpsc::Sender<Output>,
}

impl InferenceEngine {
    pub fn new(feature_rx: mpsc::Receiver<Features>, output_tx: mpsc::Sender<Output>) -> Result<Self> {

        let environment = Environment::builder()
        .with_name("market_regime_inference")
        .build()?;

        Ok(Self {
            environment,
            feature_rx,
            output_tx,
        })
    }

    pub async fn run_inference(&mut self) -> Result<()> {
        let mut session = self.environment
            .new_session_builder()?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_number_threads(2)?
            .with_model_from_file("../model/MarketRegimeModel.onnx")?;

        while let Some(features) = self.feature_rx.recv().await {
            let input_data = vec![features.price_change, features.price_momentum, features.price_volatility, features.volume_ratio, features.volume_momentum, features.price_acceleration];
            let input_array = Array2::from_shape_vec((1, NUM_FEATURES), input_data)?;
            let outputs = session.run(vec![input_array.into_dyn()])?; 
            let output_tensor = &outputs[0];

            // Convert tensor to Rust Vec<f32>
            let output_vec: Vec<f64> = output_tensor
                .as_slice().expect("Cannot convert output tensor to slice")
                .to_vec();
            let output = Output {
                buy_prob: output_vec[0],
                sell_prob: output_vec[1],
            };
            self.output_tx.send(output).await?;
        }
        Ok(())
    }
}