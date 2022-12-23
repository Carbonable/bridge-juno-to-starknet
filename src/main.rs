use std::sync::Arc;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use bridge_juno_to_starknet_backend::{
    domain::{BridgeRequest, TransactionRepository},
    infrastructure::juno::JunoLcd,
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

#[derive(Parser, Debug)]
struct Args {
    /// Blockchain REST endpoint
    #[arg(short, long)]
    juno_lcd: String,
}

#[derive(Debug)]
struct Config {
    juno_lcd: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Config {
                juno_lcd: String::from(&args.juno_lcd),
            }))
            .service(bridge)
            .service(health)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
