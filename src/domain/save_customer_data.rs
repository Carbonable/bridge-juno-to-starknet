use async_trait::async_trait;
use core::fmt::{Debug, Formatter};
use serde_derive::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SaveCustomerDataRequest {
    pub keplr_wallet_pubkey: String,
    pub project_id: String,
    pub token_ids: Vec<String>,
}

impl SaveCustomerDataRequest {
    pub fn new(keplr_wallet_pubkey: &str, project_id: &str, token_ids: Vec<&str>) -> Self {
        let mut tokens = vec![];
        for t in token_ids {
            tokens.push(t.into());
        }
        Self {
            keplr_wallet_pubkey: keplr_wallet_pubkey.into(),
            project_id: project_id.into(),
            token_ids: tokens,
        }
    }
}

#[derive(Debug)]
pub struct CustomerKeys {
    pub keplr_wallet_pubkey: String,
    pub project_id: String,
    pub token_ids: Vec<String>,
}

#[async_trait]
pub trait DataRepository {
    async fn save_customer_keys(&self, keys: CustomerKeys) -> Result<(), SaveCustomerDataError>;
    async fn get_customer_keys(
        &self,
        keplr_wallet_pubkey: &str,
        project_id: &str,
    ) -> Result<CustomerKeys, SaveCustomerDataError>;
}

impl Debug for dyn DataRepository {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "SignedHashValidator{{}}")
    }
}

pub enum SaveCustomerDataError {
    NotImpled,
    NotFound,
    FailedToPersistToDatabase,
}

pub async fn handle_save_customer_data(
    req: &SaveCustomerDataRequest,
    data_repository: Arc<dyn DataRepository>,
) -> Result<(), SaveCustomerDataError> {
    let saved = match data_repository
        .save_customer_keys(CustomerKeys {
            keplr_wallet_pubkey: req.keplr_wallet_pubkey.clone(),
            project_id: req.project_id.clone(),
            token_ids: req.token_ids.clone(),
        })
        .await
    {
        Err(e) => return Err(SaveCustomerDataError::FailedToPersistToDatabase),
        Ok(_) => (),
    };

    Ok(())
}
