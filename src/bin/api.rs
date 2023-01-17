use actix_cors::Cors;
use actix_web::{get, http, post, web, App, HttpServer, Responder};
use bridge_juno_to_starknet_backend::{
    domain::{
        bridge::{
            handle_bridge_request, BridgeError, BridgeRequest, SignedHashValidator,
            SignedHashValidatorError,
        },
        save_customer_data::{
            handle_save_customer_data, SaveCustomerDataError, SaveCustomerDataRequest,
        },
    },
    infrastructure::{
        app::{configure_application, Args, Config},
        juno::JunoLcd,
        logger::configure_logger,
        starknet::OnChainStartknetManager,
    },
};
use clap::Parser;
use futures::executor::block_on;
use log::info;
use serde_derive::Serialize;
use std::sync::Arc;

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

struct KeplrSignatureVeirfier {}
impl SignedHashValidator for KeplrSignatureVeirfier {
    fn verify(
        &self,
        signed_hash: &bridge_juno_to_starknet_backend::domain::bridge::SignedHash,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
    ) -> Result<String, bridge_juno_to_starknet_backend::domain::bridge::SignedHashValidatorError>
    {
        let pubkey = signed_hash.pub_key.key_value.to_string();
        let signature = verify_keplr_sign::Signature {
            pub_key: verify_keplr_sign::PublicKey {
                sig_type: signed_hash.pub_key.key_type.to_string(),
                sig_value: pubkey.to_string(),
            },
            signature: signed_hash.signature.to_string(),
        };

        let is_signature_ok = verify_keplr_sign::verify_arbitrary(
            keplr_wallet_pubkey,
            &pubkey,
            starknet_account_addrr.as_bytes(),
            &signature,
        );

        if !is_signature_ok {
            return Err(SignedHashValidatorError::FailedToVerifyHash);
        }

        Ok(signature.signature)
    }
}

#[post("/bridge")]
async fn bridge(req: web::Json<BridgeRequest>, data: web::Data<Config>) -> impl Responder {
    info!(
        "POST - /bridge - {} - {:#?}",
        &req.keplr_wallet_pubkey, &req.tokens_id
    );

    let provider = &data.clone().starknet_provider;

    let transaction_repository = Arc::new(JunoLcd::new(&data.clone().juno_lcd));
    let hash_validator = Arc::new(KeplrSignatureVeirfier {});
    let starknet_manager = Arc::new(OnChainStartknetManager::new(
        provider.clone(),
        &data.clone().starknet_admin_address,
        &data.clone().starknet_private_key,
        data.chain_id,
    ));

    let response = match handle_bridge_request(
        &req,
        &data.juno_admin_address,
        &data.starknet_admin_address,
        hash_validator.clone(),
        transaction_repository.clone(),
        starknet_manager.clone(),
        data.data_repository.clone(),
        data.queue_manager.clone(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => match e {
            BridgeError::InvalidSign => {
                return (
                    web::Json(ApiResponse::bad_request("Invalid sign")),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::JunoBlockChainServerError(e) => {
                return (
                    web::Json(ApiResponse::bad_request(
                        format!("Juno blockchain error {}", e.to_string().as_str()).as_str(),
                    )),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
            BridgeError::JunoBalanceIsNotZero => {
                return (
                    web::Json(ApiResponse::bad_request(
                        "Juno tokens have not been transferred yet",
                    )),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::FetchTokenError(_) => {
                return (
                    web::Json(ApiResponse::bad_request(
                        "Failed to fetch tokens from customer wallet",
                    )),
                    http::StatusCode::NOT_FOUND,
                );
            }
            BridgeError::TokenNotTransferedToAdmin(_) => {
                return (
                    web::Json(ApiResponse::bad_request("Token not transferred to admin")),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::TokenDidNotBelongToWallet(_) => {
                return (
                    web::Json(ApiResponse::bad_request(
                        "Token did not belong to provided wallet.",
                    )),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::TokenAlreadyMinted(_) => {
                return (
                    web::Json(ApiResponse::bad_request("Token has already been minted")),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::ErrorWhileMintingToken => {
                return (
                    web::Json(ApiResponse::bad_request("Error while minting token")),
                    http::StatusCode::BAD_REQUEST,
                );
            }
            BridgeError::EnqueueingIssue => {
                return (
                    web::Json(ApiResponse::bad_request(
                        "Error while enqueing your token for minting",
                    )),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        },
    };
    let mut http_status = http::StatusCode::OK;
    for (_token, (_msg, err)) in response.checks.iter() {
        http_status = match err {
            None => break,
            Some(s) => match s.as_str() {
                "Failed to fecth token data from juno chain." => http::StatusCode::BAD_REQUEST,
                "Juno node responded with an error status please try again later" => {
                    http::StatusCode::INTERNAL_SERVER_ERROR
                }
                "Transaction not found on chain." => http::StatusCode::NOT_FOUND,
                // Catching everything into BAD_REQUEST, only handle the other cases.
                _ => http::StatusCode::BAD_REQUEST,
            },
        };
    }

    (
        web::Json(ApiResponse {
            error: None,
            message: "".into(),
            code: match http_status {
                http::StatusCode::OK => 200,
                http::StatusCode::BAD_REQUEST => 400,
                http::StatusCode::NOT_FOUND => 404,
                http::StatusCode::INTERNAL_SERVER_ERROR => 500,
                _ => 200,
            },
            body: Some(response),
        }),
        http_status,
    )
}

#[get("/health")]
async fn health() -> impl Responder {
    info!("GET - /health");
    ("I'm ok !", http::StatusCode::OK)
}

#[post("/customer/data")]
async fn save_customer_tokens(
    request: web::Json<SaveCustomerDataRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    info!(
        "POST - /customer/data - {} - {}",
        &request.keplr_wallet_pubkey, &request.project_id
    );

    let _res = match handle_save_customer_data(&request, config.data_repository.clone()).await {
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

#[get("/customer/data/{keplr_wallet_pubkey}/{project_id}")]
async fn get_customer_migration_state(
    path: web::Path<(String, String)>,
    data: web::Data<Config>,
) -> impl Responder {
    let (keplr_wallet_pubkey, project_id) = path.into_inner();
    let queue_manager = data.clone().queue_manager.clone();
    let res = queue_manager
        .get_customer_migration_state(&keplr_wallet_pubkey, &project_id)
        .await;

    let mut status_code = http::StatusCode::OK;
    if res.len() == 0 {
        status_code = http::StatusCode::NOT_FOUND;
    }

    (web::Json(res), status_code)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    configure_logger();
    info!("Starting bridge application.");

    let args = Args::parse();

    info!("Ready to handle requests.");

    HttpServer::new(move || {
        let config = block_on(configure_application(&args));
        let cors = Cors::default()
            .allowed_origin(&args.frontend_uri.as_str())
            .allowed_methods(vec!["POST"])
            .allowed_headers(vec![http::header::CONTENT_TYPE]);
        App::new()
            .app_data(web::Data::new(config))
            .wrap(cors)
            .service(health)
            .service(bridge)
            .service(save_customer_tokens)
            .service(get_customer_migration_state)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
