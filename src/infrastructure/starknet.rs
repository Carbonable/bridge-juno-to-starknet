use async_trait::async_trait;
use log::{error, info};
use starknet::{
    accounts::{Account, Call, SingleOwnerAccount},
    core::{
        chain_id,
        types::{AddTransactionResult, BlockId, CallFunction, FieldElement, TransactionStatus},
    },
    macros::selector,
    providers::{Provider, SequencerGatewayProvider},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::domain::bridge::{MintError, MintTransactionResult, StarknetManager};

const TRANSACTION_CHECK_MAX_RETRY: u8 = 30;
const TRANSACTION_CHECK_WAIT_TIME: u64 = 5;

pub struct OnChainStartknetManager {
    provider: Arc<SequencerGatewayProvider>,
    account_address: String,
    account_private_key: String,
}

impl OnChainStartknetManager {
    pub fn new(
        provider: Arc<SequencerGatewayProvider>,
        account_addr: &str,
        account_pk: &str,
    ) -> Self {
        Self {
            provider,
            account_address: account_addr.to_string(),
            account_private_key: account_pk.to_string(),
        }
    }

    async fn check_transaction_status(&self, tx_result: &AddTransactionResult) -> Option<String> {
        info!(
            "Checking transaction status : {}",
            hex::encode(tx_result.transaction_hash.to_bytes_be())
        );
        let provider = self.provider.clone();
        let mut retry_count = 0;
        while TRANSACTION_CHECK_MAX_RETRY >= retry_count {
            retry_count += 1;
            let tx_status_info = &provider
                .get_transaction_status(
                    FieldElement::from_dec_str(&tx_result.transaction_hash.to_string()).unwrap(),
                )
                .await;

            if tx_status_info.is_err() {
                sleep(Duration::from_secs(TRANSACTION_CHECK_WAIT_TIME)).await;
                continue;
            }

            let tx = tx_status_info.as_ref().unwrap();
            if TransactionStatus::Rejected == tx.status {
                return match &tx.transaction_failure_reason {
                    Some(fr) => Some(fr.code.to_string()),
                    None => None,
                };
            }
            if TransactionStatus::AcceptedOnL2 == tx.status
                || TransactionStatus::AcceptedOnL1 == tx.status
            {
                info!(
                    "Transaction with hash {}, has status : {:#?}",
                    hex::encode(tx_result.transaction_hash.to_bytes_be()),
                    tx.status
                );
                return None;
            }

            sleep(Duration::from_secs(TRANSACTION_CHECK_WAIT_TIME)).await;
            continue;
        }

        return None;
    }
}

#[async_trait]
impl StarknetManager for OnChainStartknetManager {
    async fn project_has_token(&self, project_id: &str, token_id: &str) -> bool {
        let provider = self.provider.clone();
        info!(
            "Checking if project {} has token id {} minted",
            project_id, token_id
        );
        let res = provider
            .call_contract(
                CallFunction {
                    contract_address: FieldElement::from_hex_be(project_id).unwrap(),
                    entry_point_selector: selector!("ownerOf"),
                    calldata: vec![
                        FieldElement::from_dec_str(token_id).unwrap(),
                        FieldElement::ZERO,
                    ],
                },
                BlockId::Latest,
            )
            .await;

        res.is_ok()
    }

    async fn mint_project_token(
        &self,
        project_id: &str,
        token_id: &str,
        starknet_account_addr: &str,
    ) -> Result<MintTransactionResult, MintError> {
        info!(
            "Trying to mint token {} on project {}",
            token_id, project_id
        );
        let provider = self.provider.clone();
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(self.account_private_key.as_str()).unwrap(),
        ));

        let address = FieldElement::from_hex_be(self.account_address.as_str()).unwrap();
        let to = FieldElement::from_hex_be(starknet_account_addr).unwrap();

        let account = SingleOwnerAccount::new(provider, signer, address, chain_id::TESTNET);

        let res = account
            .execute(&[Call {
                to: FieldElement::from_hex_be(project_id).unwrap(),
                selector: selector!("mint"),
                calldata: vec![
                    to,
                    FieldElement::from_dec_str(token_id).unwrap(),
                    FieldElement::ZERO,
                ],
            }])
            .send()
            .await;

        match res {
            Ok(tx) => {
                info!(
                    "Token id {} minting in progress -> #{}",
                    token_id,
                    hex::encode(tx.transaction_hash.to_bytes_be())
                );

                let tx_status_info = self.check_transaction_status(&tx).await;

                Ok((
                    hex::encode(tx.transaction_hash.to_bytes_be()),
                    tx_status_info,
                ))
            }
            Err(e) => {
                error!(
                    "Error while minting token id {} -> {}",
                    token_id,
                    e.to_string()
                );
                Err(MintError::Failure)
            }
        }
    }
}
