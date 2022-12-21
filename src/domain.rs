use core::fmt::{Debug, Formatter};
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug)]
pub struct BridgeRequest {
    pub signed_hash: String,
    pub starknet_account_addr: String,
    pub keplr_wallet_pubkey: String,
    pub project_id: String,
    pub tokens_id: Vec<String>,
}

impl BridgeRequest {
    pub fn new(
        signed_hash: &str,
        starknet_account_addr: &str,
        keplr_wallet_pubkey: &str,
        project_id: &str,
        tokens_id: Vec<&str>,
    ) -> Self {
        let mut tokens = vec![];
        for t in tokens_id {
            tokens.push(t.into());
        }
        Self {
            signed_hash: signed_hash.into(),
            starknet_account_addr: starknet_account_addr.into(),
            keplr_wallet_pubkey: keplr_wallet_pubkey.into(),
            project_id: project_id.into(),
            tokens_id: tokens,
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
pub struct Message {
    pub msg: MsgTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub contract: String,
    pub messages: Message,
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
}

pub enum SignedHashValidatorError {
    FailedToVerifyHash,
}

pub type BridgeResponse = Result<Vec<String>, BridgeError>;

pub trait SignedHashValidator {
    fn verify(
        &self,
        signed_hash: &str,
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
}

pub trait TransactionRepository {
    fn get_transactions_for_contract(
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

pub enum MintError {}
pub trait StarknetManager {
    fn project_has_token(&self, project_id: &str, token_id: &str) -> bool;
    fn mint_project_token(
        &self,
        project_id: &str,
        token_id: &str,
        starknet_account_addr: &str,
    ) -> Result<String, MintError>;
}
impl Debug for dyn StarknetManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "StarknetManager{{}}")
    }
}

pub fn handle_bridge_request(
    req: &BridgeRequest,
    keplr_admin_wallet: &str,
    hash_validator: Arc<dyn SignedHashValidator>,
    transaction_repository: Arc<dyn TransactionRepository>,
    starknet_manager: Arc<dyn StarknetManager>,
) -> BridgeResponse {
    let hash = match hash_validator.verify(
        &req.signed_hash,
        &req.starknet_account_addr,
        &req.keplr_wallet_pubkey,
    ) {
        Ok(h) => h,
        Err(_err) => return Err(BridgeError::InvalidSign),
    };

    let mut minted_tokens = Vec::new();
    // Should return an array of transactions for given token
    for token in &req.tokens_id {
        let transactions =
            transaction_repository.get_transactions_for_contract(&req.project_id, token.as_str());
        if transactions.is_err() {
            return Err(BridgeError::FetchTokenError(token.to_string().into()));
        }
        if let Ok(t) = transactions {
            // Last transaction at index 0 should have admin wallet as recipient
            // transaction at index 1 should have customer keplr wallet as recipient
            let admin_transfert = match &t[0].messages.msg {
                MsgTypes::TransferNft(t) => t,
            };
            let prev_owner = match &t[1].messages.msg {
                MsgTypes::TransferNft(t) => t,
            };
            if admin_transfert.recipient != keplr_admin_wallet {
                return Err(BridgeError::TokenNotTransferedToAdmin(token.to_string()));
            }
            if prev_owner.recipient != req.keplr_wallet_pubkey {
                return Err(BridgeError::TokenDidNotBelongToWallet(token.to_string()));
            }

            // If token has already been minted, customer needs to know
            if starknet_manager.project_has_token(&req.project_id, token) {
                return Err(BridgeError::TokenAlreadyMinted(token.to_string()));
            }
            // Mint token on starknet
            let mint = starknet_manager.mint_project_token(
                &req.project_id,
                token,
                &req.starknet_account_addr,
            );
            match mint {
                Ok(m) => minted_tokens.push(m),
                Err(_) => return Err(BridgeError::ErrorWhileMintingToken),
            }
        }
    }

    Ok(minted_tokens)
}
