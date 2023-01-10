use async_trait::async_trait;
use log::{error, info};
use starknet::{
    accounts::{Account, AccountCall, Call, SingleOwnerAccount},
    core::{
        chain_id,
        types::{BlockId, CallFunction, FieldElement},
    },
    macros::selector,
    providers::{Provider, SequencerGatewayProvider},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;

use crate::domain::bridge::{MintError, StarknetManager};

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

        let account = SingleOwnerAccount::new(provider, signer, address, chain_id::TESTNET);
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
}
