use async_trait::async_trait;
use core::fmt::{Debug, Formatter};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::save_customer_data::DataRepository;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct PubKey {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub key_type: String,
    #[serde(rename(serialize = "value", deserialize = "value"))]
    pub key_value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignedHash {
    pub pub_key: PubKey,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct BridgeRequest {
    pub signed_hash: SignedHash,
    pub starknet_account_addr: String,
    pub starknet_project_addr: String,
    pub keplr_wallet_pubkey: String,
    pub project_id: String,
    pub tokens_id: Option<Vec<String>>,
}

impl BridgeRequest {
    pub fn new(
        signed_hash: SignedHash,
        starknet_account_addr: &str,
        starknet_project_addr: &str,
        keplr_wallet_pubkey: &str,
        project_id: &str,
        tokens_id: Vec<&str>,
    ) -> Self {
        let mut tokens = vec![];
        for t in tokens_id {
            tokens.push(t.into());
        }
        Self {
            signed_hash,
            starknet_account_addr: starknet_account_addr.into(),
            starknet_project_addr: starknet_project_addr.into(),
            keplr_wallet_pubkey: keplr_wallet_pubkey.into(),
            project_id: project_id.into(),
            tokens_id: Some(tokens),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferNft {
    pub recipient: String,
    pub token_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MsgTypes {
    TransferNft(TransferNft),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub contract: String,
    pub msg: MsgTypes,
    pub sender: String,
}

#[derive(Debug)]
pub enum BridgeError {
    InvalidSign,
    JunoBalanceIsNotZero,
    FetchTokenError(String),
    TokenNotTransferedToAdmin(String),
    TokenDidNotBelongToWallet(String),
    TokenAlreadyMinted(String),
    ErrorWhileMintingToken,
    JunoBlockChainServerError(u16),
    EnqueueingIssue,
}

pub enum SignedHashValidatorError {
    FailedToVerifyHash,
}

pub trait SignedHashValidator {
    fn verify(
        &self,
        signed_hash: &SignedHash,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
    ) -> Result<String, SignedHashValidatorError>;
}

impl Debug for dyn SignedHashValidator {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "SignedHashValidator{{}}")
    }
}

#[derive(Debug)]
pub enum TransactionFetchError {
    FetchError(String),
    DeserializationFailed,
    JunoBlockchainServerError(u16),
}

#[async_trait]
pub trait TransactionRepository {
    async fn get_transactions_for_contract(
        &self,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<Transaction>, TransactionFetchError>;
}

impl Debug for dyn TransactionRepository {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "TransactionRepository{{}}")
    }
}

#[derive(Debug)]
pub enum QueueError {
    FailedToGetBatch,
    FailedToEnqueue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum QueueStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueueItem {
    pub id: Option<Uuid>,
    pub keplr_wallet_pubkey: String,
    pub starknet_wallet_pubkey: String,
    pub project_id: String,
    pub token_id: String,
    pub status: QueueStatus,
    pub transaction_hash: Option<String>,
}

impl QueueItem {
    pub fn new(pubkey: &str, starknet_pubkey: &str, project_id: &str, token: String) -> Self {
        Self {
            id: None,
            keplr_wallet_pubkey: pubkey.into(),
            starknet_wallet_pubkey: starknet_pubkey.into(),
            project_id: project_id.into(),
            token_id: token,
            status: QueueStatus::Pending,
            transaction_hash: None,
        }
    }
}

#[derive(Debug)]
pub enum QueueUpdateError {
    StatusUpdateFail(Vec<String>),
}

#[async_trait]
pub trait QueueManager {
    async fn enqueue(
        &self,
        keplr_wallet_pubkey: &str,
        starknet_wallet_pubkey: &str,
        project_id: &str,
        token_ids: Vec<String>,
    ) -> Result<Vec<QueueItem>, QueueError>;
    async fn get_batch(&self) -> Result<Vec<QueueItem>, QueueError>;
    async fn get_customer_migration_state(
        &self,
        keplr_wallet_pubkey: &str,
        project_id: &str,
    ) -> Vec<QueueItem>;
    async fn update_queue_items_status(
        &self,
        ids: &Vec<String>,
        transaction_hash: String,
        status: QueueStatus,
    ) -> Result<(), QueueUpdateError>;
}

impl Debug for dyn QueueManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "QueueManager{{}}")
    }
}

pub enum MintError {
    Failure,
}

// First string is transaction_hash while second is the optionnal error result
pub type MintTransactionResult = (String, Option<String>);

#[async_trait]
pub trait StarknetManager {
    async fn project_has_token(&self, project_id: &str, token_id: &str) -> bool;
    async fn mint_project_token(
        &self,
        project_id: &str,
        tokens: &[String],
        starknet_account_addr: &str,
    ) -> Result<String, MintError>;
    async fn batch_mint_tokens(
        &self,
        project_id: &str,
        queue_items: Vec<QueueItem>,
    ) -> Result<(String, QueueStatus), MintError>;
}
impl Debug for dyn StarknetManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "StarknetManager{{}}")
    }
}

type MintPreChecks = HashMap<String, (String, Option<String>)>;
// Represents the response as [token_ids], Transaction hash
type MintResult = (Vec<String>, String);

#[derive(Serialize, Deserialize, Debug)]
pub struct BridgeResponse {
    pub checks: MintPreChecks,
    pub result: MintResult,
}
pub async fn handle_bridge_request<'a, 'b, 'c, 'd, 'e>(
    req: &BridgeRequest,
    keplr_admin_wallet: &str,
    starknet_admin_address: &str,
    hash_validator: Arc<dyn SignedHashValidator + 'a>,
    transaction_repository: Arc<dyn TransactionRepository + 'b>,
    starknet_manager: Arc<dyn StarknetManager + 'c>,
    data_repository: Arc<dyn DataRepository + 'd>,
    queue_manager: Arc<dyn QueueManager + 'e>,
) -> Result<BridgeResponse, BridgeError> {
    match hash_validator.verify(
        &req.signed_hash,
        &starknet_admin_address,
        &req.keplr_wallet_pubkey,
    ) {
        Ok(h) => h,
        Err(_err) => return Err(BridgeError::InvalidSign),
    };

    // Fetch token from wallet id from database
    let tokens = match data_repository
        .get_customer_keys(&req.keplr_wallet_pubkey, &req.project_id)
        .await
    {
        Ok(t) => Some(t.token_ids),
        Err(_) => None,
    };

    if tokens.is_none() && req.tokens_id.as_ref().unwrap().len() == 0 {
        error!(
            "No tokens ids found for wallet {} and project {}",
            &req.keplr_wallet_pubkey, &req.project_id
        );
        return Err(BridgeError::FetchTokenError(
            "Failed to fetch tokens from database".into(),
        ));
    }

    if let Some(req_token) = &req.tokens_id {
        let token_ids = match req_token.len() {
            0 => tokens.unwrap(),
            _ => req_token.to_vec(),
        };

        info!("Migrating tokens : [{}]", token_ids.join(", "));
        let mut checked_tokens = HashMap::new();
        for token in &token_ids {
            let transactions = transaction_repository
                .get_transactions_for_contract(&req.project_id, token.as_str())
                .await;
            if transactions.is_err() {
                match transactions.unwrap_err() {
                    TransactionFetchError::FetchError(_) => {
                        checked_tokens.insert(
                            token.to_string(),
                            (
                                token.to_string(),
                                Some("Failed to fecth token data from juno chain.".into()),
                            ),
                        );
                        continue;
                    }
                    TransactionFetchError::DeserializationFailed => {
                        checked_tokens.insert(
                            token.to_string(),
                            (
                                token.to_string(),
                                Some("Failed to deserialize data from juno blockchain".into()),
                            ),
                        );
                        continue;
                    }
                    TransactionFetchError::JunoBlockchainServerError(_e) => {
                        checked_tokens.insert(token.to_string(),(
                        token.to_string(),
                        Some("Juno node responded with an error status please try again later".into()),
                    ));
                        continue;
                    }
                };
            }

            if let Ok(t) = transactions {
                if 0 == t.len() {
                    error!(
                        "No transactions found on juno chain for wallet {} and project {}",
                        &req.keplr_wallet_pubkey, &req.project_id
                    );
                    checked_tokens.insert(
                        token.to_string(),
                        (
                            token.to_string(),
                            Some("Transaction not found on chain.".into()),
                        ),
                    );
                    continue;
                }
                // Last transaction at index 0 should have admin wallet as recipient
                // Only checking transaction at index 0 as this is the last transaction done
                // on given token.
                let admin_transfert = match &t[0].msg {
                    MsgTypes::TransferNft(t) => t,
                };

                if admin_transfert.recipient != keplr_admin_wallet {
                    error!(
                        "Token id {} last owner is not admin : {}",
                        token, keplr_admin_wallet
                    );
                    checked_tokens.insert(
                        token.to_string(),
                        (
                            token.to_string(),
                            Some("Token was not transfered to admin".into()),
                        ),
                    );
                    continue;
                }
                if t[0].sender != req.keplr_wallet_pubkey {
                    error!(
                        "Token id {} sender does not match given wallet pubkey {}",
                        token, req.keplr_wallet_pubkey
                    );
                    checked_tokens.insert(
                        token.to_string(),
                        (
                            token.to_string(),
                            Some("Token sender didn't match customer wallet public key".into()),
                        ),
                    );
                    continue;
                }

                // If token has already been minted, customer needs to know
                if starknet_manager
                    .project_has_token(&req.starknet_project_addr, token)
                    .await
                {
                    error!("Token id {} has already been minted", token);
                    checked_tokens.insert(
                        token.to_string(),
                        (
                            token.to_string(),
                            Some("Token has already been minted".into()),
                        ),
                    );
                    continue;
                }

                checked_tokens.insert(token.to_string(), (token.to_string(), None));
            }
        }

        let mut token_to_mint = Vec::new();
        for (token, (_msg, err)) in checked_tokens.iter() {
            if err.is_none() {
                token_to_mint.push(token.to_string());
            }
        }
        let _queue_items = match queue_manager
            .enqueue(
                &req.keplr_wallet_pubkey,
                &req.starknet_account_addr,
                &req.starknet_project_addr,
                token_to_mint.clone(),
            )
            .await
        {
            Ok(qi) => qi,
            Err(e) => match e {
                _ => return Err(BridgeError::EnqueueingIssue),
            },
        };

        return Ok(BridgeResponse {
            checks: checked_tokens,
            result: (
                token_to_mint.iter().map(|t| t.to_string()).collect(),
                "Your token(s) migration have been queued in. You can stay on this page to check the queueing status.".to_string(),
            ),
        });
    }

    Err(BridgeError::FetchTokenError(
        "Failed to fetch tokens from database".into(),
    ))
}
