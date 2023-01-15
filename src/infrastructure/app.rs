use super::postgresql::{get_connection, PostgresDataRepository, PostgresQueueManager};
use crate::domain::{bridge::QueueManager, save_customer_data::DataRepository};
use clap::Parser;
use starknet::{core::types::FieldElement, providers::SequencerGatewayProvider};
use std::sync::Arc;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Blockchain REST endpoint
    #[arg(long, env = "JUNO_LCD")]
    pub juno_lcd: String,
    /// Database url to connect to
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,
    /// Juno admin wallet address
    #[arg(long, env = "JUNO_ADMIN_ADDRESS")]
    pub juno_admin_address: String,
    /// Starknet admin wallet address
    #[arg(long, env = "STARKNET_ADMIN_ADDRESS")]
    pub starknet_admin_address: String,
    /// Starknet admin wallet private key
    #[arg(long, env = "STARKNET_ADMIN_PRIVATE_KEY")]
    pub starknet_admin_private_key: String,
    /// Starknet network id
    #[arg(long, env = "STARKNET_NETWORK_ID")]
    pub starknet_network_id: String,
    /// Starknet network id
    #[arg(long, env = "FRONTEND_URI")]
    pub frontend_uri: String,
    /// Queue batch size
    #[arg(long, env = "BATCH_SIZE")]
    pub batch_size: u8,
}

pub struct Config {
    pub juno_lcd: String,
    pub database_url: String,
    pub data_repository: Arc<dyn DataRepository>,
    pub queue_manager: Arc<dyn QueueManager>,
    pub starknet_provider: Arc<SequencerGatewayProvider>,
    pub juno_admin_address: String,
    pub starknet_admin_address: String,
    pub starknet_private_key: String,
    pub frontend_uri: String,
    pub chain_id: FieldElement,
}

pub async fn configure_application(args: &Args) -> Config {
    let connection = match get_connection(&args.database_url).await {
        Ok(c) => Arc::new(c),
        Err(e) => panic!("Failed to connect to database error : {}", e),
    };

    let provider = match args.starknet_network_id.as_str() {
        "mainnet" => Arc::new(SequencerGatewayProvider::starknet_alpha_mainnet()),
        "testnet-1" => Arc::new(SequencerGatewayProvider::starknet_alpha_goerli()),
        "devnet-1" => Arc::new(SequencerGatewayProvider::starknet_nile_localhost()),
        _ => panic!("Starknet provider is not allowed"),
    };
    let chain_id = match args.starknet_network_id.as_str() {
        "mainnet" => starknet::core::chain_id::MAINNET,
        "testnet-1" => starknet::core::chain_id::TESTNET,
        "devnet-1" => starknet::core::chain_id::TESTNET2,
        _ => panic!("Starknet chain_id is not allowed"),
    };

    let data_repository = Arc::new(PostgresDataRepository::new(connection.clone()));
    let queue_manager = Arc::new(PostgresQueueManager::new(
        connection.clone(),
        args.batch_size,
    ));

    Config {
        juno_lcd: String::from(&args.juno_lcd),
        database_url: String::from(&args.database_url),
        data_repository: data_repository.clone(),
        queue_manager: queue_manager.clone(),
        juno_admin_address: String::from(&args.juno_admin_address),
        starknet_admin_address: String::from(&args.starknet_admin_address),
        starknet_private_key: String::from(&args.starknet_admin_private_key),
        starknet_provider: provider.clone(),
        frontend_uri: String::from(&args.frontend_uri),
        chain_id,
    }
}
