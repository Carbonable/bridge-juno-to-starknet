use log::info;
use starknet::providers::SequencerGatewayProvider;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{get, http, post, web, App, HttpResponse, HttpServer, Responder};
use bridge_juno_to_starknet_backend::{
    domain::{
        bridge::{handle_bridge_request, BridgeError, BridgeRequest, SignedHashValidator},
        save_customer_data::{
            handle_save_customer_data, DataRepository, SaveCustomerDataError,
            SaveCustomerDataRequest,
        },
    },
    infrastructure::{
        juno::JunoLcd,
        postgresql::{get_connection, PostgresDataRepository},
        starknet::OnChainStartknetManager,
    },
};
use clap::Parser;
use serde_derive::Serialize;

#[derive(Serialize)]
struct ApiResponse<T> {
    error: Option<String>,
    message: String,
    code: u32,
    body: Option<T>,
}

impl<T> ApiResponse<T> {
    fn create(error: Option<&str>, message: &str, code: u32, body: Option<T>) -> Self {
        let err = match error {
            Some(e) => Some(e.to_string()),
            None => None,
        };
        Self {
            error: err,
            message: message.into(),
            code,
            body,
        }
    }

    fn bad_request(message: &str) -> Self {
        ApiResponse::create(Some("Bad Request"), message, 400, None)
    }
}

// Boilerplate code to replace with real implementation
// @todo: implement real secp256k1 real signature verification
struct AlwaysTrueSignatureVerifier {}

impl SignedHashValidator for AlwaysTrueSignatureVerifier {
    fn verify(
        &self,
        signed_hash: &bridge_juno_to_starknet_backend::domain::bridge::SignedHash,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
    ) -> Result<String, bridge_juno_to_starknet_backend::domain::bridge::SignedHashValidatorError>
    {
        Ok(signed_hash.signature.to_string())
    }
}

#[post("/bridge")]
async fn bridge(req: web::Json<BridgeRequest>, data: web::Data<Config>) -> impl Responder {
    let provider = Arc::new(SequencerGatewayProvider::starknet_alpha_goerli());

    let transaction_repository = Arc::new(JunoLcd::new(&data.clone().juno_lcd));
    let hash_validator = Arc::new(AlwaysTrueSignatureVerifier {});
    let starknet_manager = Arc::new(OnChainStartknetManager::new(
        provider.clone(),
        &data.clone().starknet_admin_address,
        &data.clone().starknet_private_key,
    ));

    match handle_bridge_request(
        &req,
        &data.starknet_admin_address,
        hash_validator.clone(),
        transaction_repository.clone(),
        starknet_manager.clone(),
        data.data_repository.clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => match e {
            bridge_juno_to_starknet_backend::domain::bridge::BridgeError::InvalidSign => {
                return web::Json(ApiResponse::bad_request("Invalid sign"));
            }
            BridgeError::JunoBalanceIsNotZero => {
                return web::Json(ApiResponse::bad_request(
                    "Juno tokens have not been transferred yet",
                ));
            }
            BridgeError::FetchTokenError(_) => {
                return web::Json(ApiResponse::bad_request(
                    "Failed to fetch tokens from customer wallet",
                ));
            }
            BridgeError::TokenNotTransferedToAdmin(_) => {
                return web::Json(ApiResponse::bad_request("Token not transferred to admin"));
            }
            BridgeError::TokenDidNotBelongToWallet(_) => {
                return web::Json(ApiResponse::bad_request(
                    "Token did not belong to provided wallet.",
                ));
            }
            BridgeError::TokenAlreadyMinted(_) => {
                return web::Json(ApiResponse::bad_request("Token has already been minted"));
            }
            BridgeError::ErrorWhileMintingToken => {
                return web::Json(ApiResponse::bad_request("Error while minting token"));
            }
        },
    };

    web::Json(ApiResponse::<Vec<String>> {
        error: None,
        message: "".into(),
        code: 200,
        body: Some(vec![]),
    })
}

#[get("/health")]
async fn health() -> impl Responder {
    "I'm ok !"
}

#[post("/customer/data")]
async fn save_customer_tokens(
    request: web::Json<SaveCustomerDataRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    let res = match handle_save_customer_data(&request, config.data_repository.clone()).await {
        Ok(res) => res,
        Err(e) => match e {
            SaveCustomerDataError::NotImpled => {
                return (
                    web::Json(ApiResponse {
                        error: Some("Internal Server Error".into()),
                        message: "Unknown error".into(),
                        code: 500,
                        body: None,
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
            SaveCustomerDataError::NotFound => {
                return (
                    web::Json(ApiResponse {
                        error: Some("Not Found".into()),
                        message: "Customer not found".into(),
                        code: 404,
                        body: None,
                    }),
                    http::StatusCode::NOT_FOUND,
                )
            }
            SaveCustomerDataError::FailedToPersistToDatabase => {
                return (
                    web::Json(ApiResponse {
                        error: Some("Internal Server Error".into()),
                        message: "Error while saving customer to database".into(),
                        code: 500,
                        body: None,
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        },
    };

    (
        web::Json(ApiResponse::<Vec<String>> {
            error: None,
            message: "Saved customer pubkey // tokens".into(),
            code: 201,
            body: None,
        }),
        http::StatusCode::CREATED,
    )
}

#[derive(Parser, Debug)]
struct Args {
    /// Blockchain REST endpoint
    #[arg(long, env = "JUNO_LCD")]
    juno_lcd: String,
    /// Database url to connect to
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    /// Juno admin wallet address
    #[arg(long, env = "JUNO_ADMIN_ADDR")]
    juno_admin_address: String,
    /// Starknet admin wallet address
    #[arg(long, env = "STARKNET_ADMIN_ADDR")]
    starknet_admin_address: String,
    /// Starknet admin wallet private key
    #[arg(long, env = "STARKNET_ADMIN_PK")]
    starknet_admin_private_key: String,
    /// Starknet network id
    #[arg(long, env = "STARKNET_NETWORK_ID")]
    starknet_network_id: String,
    /// Starknet network id
    #[arg(long, env = "FRONTEND_URI")]
    frontend_uri: String,
}

struct Config {
    juno_lcd: String,
    data_repository: Arc<dyn DataRepository>,
    starknet_provider: Arc<SequencerGatewayProvider>,
    juno_admin_address: String,
    starknet_admin_address: String,
    starknet_private_key: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    info!("Starting bridge application.");
    let args = Args::parse();
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

    let data_repository = Arc::new(PostgresDataRepository::new(connection));

    info!("Ready to handle requests.");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(args.frontend_uri.as_str())
            .allowed_methods(vec!["POST"])
            .allowed_headers(vec![http::header::CONTENT_TYPE]);
        App::new()
            .app_data(web::Data::new(Config {
                juno_lcd: String::from(&args.juno_lcd),
                data_repository: data_repository.clone(),
                juno_admin_address: String::from(&args.juno_admin_address),
                starknet_admin_address: String::from(&args.starknet_admin_address),
                starknet_private_key: String::from(&args.starknet_admin_private_key),
                starknet_provider: provider.clone(),
            }))
            .wrap(cors)
            .service(health)
            .service(bridge)
            .service(save_customer_tokens)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
