fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/MarketDataFeed.proto"], &["proto/"])?;
    Ok(())
}
