use core::fmt::{Debug, Formatter};
use std::sync::Arc;

#[derive(Debug)]
pub struct BridgeRequest {
    signed_hash: String,
    starknet_account_addr: String,
    keplr_wallet_pubkey: String,
    project_id: String,
    tokens_id: Vec<String>,
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

#[derive(Debug)]
pub enum BridgeError {
    InvalidSign,
    JunoBalanceIsNotZero,
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

pub fn handle_bridge_request(
    req: &BridgeRequest,
    hash_validator: Arc<dyn SignedHashValidator>,
) -> BridgeResponse {
    let hash = match hash_validator.verify(
        &req.signed_hash,
        &req.starknet_account_addr,
        &req.keplr_wallet_pubkey,
    ) {
        Ok(h) => h,
        Err(_err) => return Err(BridgeError::InvalidSign),
    };

    Ok(vec!["the-new-token-1".into(), "the-new-token-2".into()])
}
