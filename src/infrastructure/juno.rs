use async_trait::async_trait;
use log::error;
use reqwest::Response;
use serde_derive::{Deserialize, Serialize};
use std::thread::sleep;
use std::time::Duration;

use crate::domain::bridge::{MsgTypes, Transaction, TransactionFetchError, TransactionRepository};

const MAX_RETRY: i32 = 5;

#[derive(Debug)]
pub enum JunoLcdError {
    ApiGetFailure(String),
    Reqwest(String),
}

pub struct JunoLcd {
    lcd_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pagination {
    next_key: Option<String>,
    total: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionItem {
    body: Body,
    signatures: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Body {
    messages: Vec<Transaction>,
    memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionResponseItem {
    messages: Vec<Transaction>,
    memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionResponse {
    height: String,
    txhash: String,
    codespace: String,
    code: u64,
    data: String,
    raw_log: String,
    info: String,
    gas_wanted: String,
    gas_used: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionApiResponse {
    txs: Vec<TransactionItem>,
    tx_responses: Vec<TransactionResponse>,
    pagination: Pagination,
}

#[async_trait]
impl TransactionRepository for JunoLcd {
    async fn get_transactions_for_contract(
        &self,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<crate::domain::bridge::Transaction>, crate::domain::bridge::TransactionFetchError>
    {
        // Hard limitting limit and offset as this is not relevant here to use it as a param.
        let endpoint = format!(
            "/cosmos/tx/v1beta1/txs?events=execute._contract_address=%27{}%27&pagination.limit=60&pagination.offset=0&pagination.count_total=true",
            project_id
        );
        let response = match self.get(endpoint).await {
            Ok(t) => t,
            Err(e) => {
                error!("fetching Juno blockchain transactions : {:#?}", e);
                return Err(TransactionFetchError::FetchError(
                    "Failed to call transaction API".into(),
                ));
            }
        };
        if 500 <= response.status().as_u16() {
            return Err(TransactionFetchError::JunoBlockchainServerError(
                response.status().into(),
            ));
        }

        let txs = match response.json::<TransactionApiResponse>().await {
            Ok(t) => t,
            Err(_e) => return Err(TransactionFetchError::DeserializationFailed),
        };

        let mut domain_tx: Vec<Transaction> = Vec::new();
        for transaction_item in txs.txs.iter() {
            for msg in transaction_item.body.messages.iter() {
                let transfer = match &msg.msg {
                    MsgTypes::TransferNft(t) => t,
                };

                if transfer.token_id == token_id {
                    domain_tx.push(msg.clone());
                }
            }
        }

        // Transaction are returned with the OLDEST in first position, so we need to reverse them
        // right away
        domain_tx.reverse();

        Ok(domain_tx)
    }
}

impl JunoLcd {
    pub fn new(lcd_address: &str) -> Self {
        Self {
            lcd_address: lcd_address.into(),
        }
    }

    async fn get(&self, endpoint: String) -> Result<Response, JunoLcdError> {
        for i in 0..MAX_RETRY {
            let addr = self.lcd_address.clone();
            if let Ok(client) = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
            {
                let request = client
                    .get(format!("{}{}", addr, endpoint.clone()))
                    .send()
                    .await;

                if request.is_err() {
                    if i < MAX_RETRY {
                        sleep(Duration::from_secs(15));
                        continue;
                    }
                    return Err(JunoLcdError::ApiGetFailure(endpoint));
                }

                return Ok(request.unwrap());
            } else {
                return Err(JunoLcdError::Reqwest("Failed to build client".into()));
            }
        }

        // Add notification here.
        Err(JunoLcdError::ApiGetFailure(endpoint))
    }
}
