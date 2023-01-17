use async_trait::async_trait;
use log::{error, info};
use starknet::{
    accounts::{Account, AccountCall, Call, SingleOwnerAccount},
    core::types::{AddTransactionResult, BlockId, CallFunction, FieldElement, TransactionStatus},
    macros::selector,
    providers::{Provider, SequencerGatewayProvider},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::domain::bridge::{MintError, QueueItem, QueueStatus, StarknetManager};

const TRANSACTION_CHECK_MAX_RETRY: u8 = 30;
const TRANSACTION_CHECK_WAIT_TIME: u64 = 5;

struct TransactionRejected(Option<String>);

pub struct OnChainStartknetManager {
    provider: Arc<SequencerGatewayProvider>,
    account_address: String,
    account_private_key: String,
    chain_id: FieldElement,
}

impl OnChainStartknetManager {
    pub fn new(
        provider: Arc<SequencerGatewayProvider>,
        account_addr: &str,
        account_pk: &str,
        chain_id: FieldElement,
    ) -> Self {
        Self {
            provider,
            account_address: account_addr.to_string(),
            account_private_key: account_pk.to_string(),
            chain_id,
        }
    }

    async fn check_transaction_status(
        &self,
        tx_result: &AddTransactionResult,
    ) -> Result<(), TransactionRejected> {
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
                    Some(fr) => Err(TransactionRejected(Some(fr.code.to_string()))),
                    None => Err(TransactionRejected(None)),
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
                return Ok(());
            }

            sleep(Duration::from_secs(TRANSACTION_CHECK_WAIT_TIME)).await;
            continue;
        }

        return Ok(());
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
        tokens: &[String],
        starknet_account_addr: &str,
    ) -> Result<String, MintError> {
        info!(
            "Trying to mint tokens {:#?} on project {}",
            tokens, project_id
        );
        let provider = self.provider.clone();
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(self.account_private_key.as_str()).unwrap(),
        ));

        let address = FieldElement::from_hex_be(self.account_address.as_str()).unwrap();
        let to = FieldElement::from_hex_be(starknet_account_addr).unwrap();

        let account = SingleOwnerAccount::new(provider, signer, address, self.chain_id);
        let mut calls = Vec::new();
        for t in tokens {
            calls.push(Call {
                to: FieldElement::from_hex_be(project_id).unwrap(),
                selector: selector!("mint"),
                calldata: vec![
                    to,
                    FieldElement::from_dec_str(t).unwrap(),
                    FieldElement::ZERO,
                ],
            })
        }

        let account_attached_call = account.execute(&calls.as_slice());

        // This value is set only to allow transactions during spike time
        let account_attached_call = account_attached_call.fee_estimate_multiplier(10.0);

        let res = account_attached_call.send().await;

        match res {
            Ok(tx) => {
                info!(
                    "Token id {:#?} minting in progress -> #{}",
                    tokens,
                    hex::encode(tx.transaction_hash.to_bytes_be())
                );

                Ok(format!(
                    "0x{}",
                    hex::encode(tx.transaction_hash.to_bytes_be())
                ))
            }
            Err(e) => {
                error!(
                    "Error while minting token id {:#?} -> {}",
                    tokens,
                    e.to_string()
                );
                Err(MintError::Failure)
            }
        }
    }
    async fn batch_mint_tokens(
        &self,
        project_id: &str,
        queue_items: Vec<QueueItem>,
    ) -> Result<(String, QueueStatus), MintError> {
        let provider = self.provider.clone();
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(self.account_private_key.as_str()).unwrap(),
        ));

        let address = FieldElement::from_hex_be(self.account_address.as_str()).unwrap();

        let account = SingleOwnerAccount::new(provider, signer, address, self.chain_id);
        let mut calls = Vec::new();
        for qi in queue_items {
            let to = FieldElement::from_hex_be(qi.starknet_wallet_pubkey.as_str()).unwrap();
            calls.push(Call {
                to: FieldElement::from_hex_be(project_id).unwrap(),
                selector: selector!("mint"),
                calldata: vec![
                    to,
                    FieldElement::from_dec_str(qi.token_id.as_str()).unwrap(),
                    FieldElement::ZERO,
                ],
            })
        }

        let account_attached_call = account.execute(&calls.as_slice());

        // This value is set only to allow transactions during spike time
        let account_attached_call = account_attached_call.fee_estimate_multiplier(10.0);

        let res = account_attached_call.send().await;

        match res {
            Ok(tx) => {
                info!(
                    "Batch transaction in progress -> #{}",
                    hex::encode(tx.transaction_hash.to_bytes_be())
                );

                let tx_hash = format!("0x{}", hex::encode(tx.transaction_hash.to_bytes_be()));
                return match self.check_transaction_status(&tx).await {
                    Err(_e) => Ok((tx_hash, QueueStatus::Error)),
                    Ok(_) => Ok((tx_hash, QueueStatus::Success)),
                };
            }
            Err(e) => {
                error!("Error while batching transaction -> {}", e.to_string());
                Err(MintError::Failure)
            }
        }
    }
}
