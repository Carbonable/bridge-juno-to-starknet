use async_trait::async_trait;
use core::fmt::{Debug, Formatter};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use super::save_customer_data::DataRepository;

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
        token_id: &str,
        starknet_account_addr: &str,
    ) -> Result<MintTransactionResult, MintError>;
}
impl Debug for dyn StarknetManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "StarknetManager{{}}")
    }
}

pub async fn handle_bridge_request<'a, 'b, 'c, 'd>(
    req: &BridgeRequest,
    keplr_admin_wallet: &str,
    hash_validator: Arc<dyn SignedHashValidator + 'a>,
    transaction_repository: Arc<dyn TransactionRepository + 'b>,
    starknet_manager: Arc<dyn StarknetManager + 'c>,
    data_repository: Arc<dyn DataRepository + 'd>,
) -> Result<HashMap<String, MintTransactionResult>, BridgeError> {
    match hash_validator.verify(
        &req.signed_hash,
        &req.starknet_account_addr,
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

    if tokens.is_none() && req.tokens_id.is_none() {
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
        let mut minted_tokens = HashMap::new();
        // Should return an array of transactions for given token
        for token in &token_ids {
            let transactions = transaction_repository
                .get_transactions_for_contract(&req.project_id, token.as_str())
                .await;
            if transactions.is_err() {
                return match transactions.unwrap_err() {
                    TransactionFetchError::FetchError(_) => {
                        Err(BridgeError::FetchTokenError(token.to_string().into()))
                    }
                    TransactionFetchError::DeserializationFailed => {
                        Err(BridgeError::FetchTokenError(token.to_string().into()))
                    }
                    TransactionFetchError::JunoBlockchainServerError(e) => {
                        Err(BridgeError::JunoBlockChainServerError(e))
                    }
                };
            }

            if let Ok(t) = transactions {
                if 0 == t.len() {
                    error!(
                        "No transactions found on juno chain for wallet {} and project {}",
                        &req.keplr_wallet_pubkey, &req.project_id
                    );
                    return Err(BridgeError::FetchTokenError(
                        "Transaction not found for token".to_string(),
                    ));
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
                    return Err(BridgeError::TokenNotTransferedToAdmin(token.to_string()));
                }
                if t[0].sender != req.keplr_wallet_pubkey {
                    error!(
                        "Token id {} sender does not match given wallet pubkey {}",
                        token, req.keplr_wallet_pubkey
                    );
                    return Err(BridgeError::TokenDidNotBelongToWallet(token.to_string()));
                }

                // If token has already been minted, customer needs to know
                if starknet_manager
                    .project_has_token(&req.starknet_project_addr, token)
                    .await
                {
                    error!("Token id {} has already been minted", token);
                    return Err(BridgeError::TokenAlreadyMinted(token.to_string()));
                }

                // Mint token on starknet
                let mint = starknet_manager
                    .mint_project_token(
                        &req.starknet_project_addr,
                        token,
                        &req.starknet_account_addr,
                    )
                    .await;

                match mint {
                    Ok(m) => minted_tokens.insert(token.to_string(), m),
                    Err(_) => {
                        return {
                            error!(
                                "Token id {} encountered an error while minting on contract {}",
                                token, req.starknet_project_addr
                            );
                            Err(BridgeError::ErrorWhileMintingToken)
                        }
                    }
                };
            }
        }

        return Ok(minted_tokens);
    }

    Err(BridgeError::FetchTokenError(
        "Failed to fetch tokens from database".into(),
    ))
}
