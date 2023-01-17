use bridge_juno_to_starknet_backend::{
    domain::consume_queue::consume_queue,
    infrastructure::{
        app::{configure_application, Args},
        logger::configure_logger,
        starknet::OnChainStartknetManager,
    },
};
use clap::Parser;
use log::{error, info};
use std::{sync::Arc, time::Instant};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    configure_logger();
    info!("Running worker");

    let args = Args::parse();
    let config = configure_application(&args).await;

    let starknet_manager = Arc::new(OnChainStartknetManager::new(
        config.starknet_provider.clone(),
        &config.starknet_admin_address,
        &config.starknet_private_key,
        config.chain_id,
    ));

    loop {
        info!("Polling new NFT's migration requests.");

        match consume_queue(config.queue_manager.clone(), starknet_manager.clone()).await {
            Ok(_) => {
                info!("Successfully handled tokens migration");
            }
            Err(_) => {
                error!("Failed to migrate tokens");
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}
