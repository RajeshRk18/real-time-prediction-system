pub mod fetcher;
pub mod logger;
pub mod config;
pub mod error;

pub mod pb {
    include!(concat!(
        env!("OUT_DIR"),
        "/com.upstox.marketdatafeederv3udapi.rpc.proto.rs"
    ));
}