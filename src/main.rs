use std::sync::Arc;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use bridge_juno_to_starknet_backend::{
    domain::{
        bridge::{BridgeRequest, TransactionRepository},
        save_customer_data::{
            handle_save_customer_data, DataRepository, SaveCustomerDataError,
            SaveCustomerDataRequest,
        },
    },
    infrastructure::{
        juno::JunoLcd,
        postgresql::{get_connection, PostgresDataRepository},
    },
};
use clap::Parser;
use serde_derive::Serialize;
use tokio_postgres::Client;

#[derive(Serialize)]
struct ApiResponse<T> {
    error: Option<String>,
    message: String,
    code: u32,
    body: Option<T>,
}

#[post("/bridge")]
async fn bridge(req: web::Json<BridgeRequest>, data: web::Data<Config>) -> impl Responder {
    let lsc = JunoLcd::new(&data.clone().juno_lcd);

    let txs = match lsc
        .get_transactions_for_contract(&req.project_id, "344")
        .await
    {
        Ok(t) => t,
        Err(_e) => {
            return web::Json(ApiResponse {
                error: Some("Internal Server Error".into()),
                message: "Error while fetching transactions on juno chain".into(),
                code: 500,
                body: None,
            })
        }
    };

    web::Json(ApiResponse {
        error: None,
        message: "".into(),
        code: 200,
        body: Some(txs),
    })
}

#[get("/health")]
async fn health() -> impl Responder {
    "Im OK !"
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
                return web::Json(ApiResponse {
                    error: Some("Internal Server Error".into()),
                    message: "Unknown error".into(),
                    code: 500,
                    body: None,
                })
            }
            SaveCustomerDataError::NotFound => {
                return web::Json(ApiResponse {
                    error: Some("Not Found".into()),
                    message: "Customer not found".into(),
                    code: 500,
                    body: None,
                })
            }
            SaveCustomerDataError::FailedToPersistToDatabase => {
                return web::Json(ApiResponse {
                    error: Some("Internal Server Error".into()),
                    message: "Error while saving customer to database".into(),
                    code: 500,
                    body: None,
                })
            }
        },
    };

    web::Json(ApiResponse::<Vec<String>> {
        error: None,
        message: "Saved customer pubkey // tokens".into(),
        code: 201,
        body: None,
    })
}

#[derive(Parser, Debug)]
struct Args {
    /// Blockchain REST endpoint
    #[arg(short, long)]
    juno_lcd: String,
}

#[derive(Debug)]
struct Config {
    juno_lcd: String,
    data_repository: Arc<dyn DataRepository>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let connection =
        match get_connection("postgresql://root:root@localhost:5432/starknet_bridge").await {
            Ok(c) => Arc::new(c),
            Err(e) => panic!("Failed to connect to database error : {}", e),
        };

    let data_repository = Arc::new(PostgresDataRepository::new(connection));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Config {
                juno_lcd: String::from(&args.juno_lcd),
                data_repository: data_repository.clone(),
            }))
            .service(bridge)
            .service(health)
            .service(save_customer_tokens)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
