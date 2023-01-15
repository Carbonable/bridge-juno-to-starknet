use bridge_juno_to_starknet_backend::infrastructure::{
    app::{configure_application, Args},
    logger::configure_logger,
};
use clap::Parser;
use log::info;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    configure_logger();
    info!("Running worker");

    let args = Args::parse();
    let config = configure_application(&args);

    loop {
        let start_time = Instant::now();
        info!("Polling new NFT's migration requests.");

        let elapsed = start_time.elapsed();
        if elapsed < Duration::from_secs(60) {
            sleep(Duration::from_secs(60 - elapsed.as_secs())).await;
        }
    }
}
