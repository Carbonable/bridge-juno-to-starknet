use std::{cell::RefCell, collections::HashMap};

use crate::domain::{
    MsgTypes, SignedHashValidator, SignedHashValidatorError, StarknetManager, Transaction,
    TransactionFetchError, TransactionRepository,
};

#[derive(Debug, Clone)]
pub struct TestSignedHashValidator {}

impl SignedHashValidator for TestSignedHashValidator {
    fn verify(
        &self,
        signed_hash: &str,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
    ) -> Result<String, SignedHashValidatorError> {
        return match signed_hash {
            "anInvalidHash" => Err(SignedHashValidatorError::FailedToVerifyHash),
            &_ => Ok(signed_hash.into()),
        };
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryTransactionRepository {
    pub transactions: RefCell<Vec<Transaction>>,
}

impl TransactionRepository for InMemoryTransactionRepository {
    fn get_transactions_for_contract(
        &self,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<Transaction>, TransactionFetchError> {
        let trans = self.transactions.borrow().clone();
        let filtered_transactions: Vec<Transaction> = trans
            .into_iter()
            .filter(|t| {
                let transfert = match &t.messages.msg {
                    MsgTypes::TransferNft(tt) => tt,
                };
                t.contract == project_id && token_id == transfert.token_id
            })
            .collect::<Vec<Transaction>>();
        Ok(filtered_transactions)
    }
}

impl InMemoryTransactionRepository {
    pub fn new(transactions: Vec<Transaction>) -> Self {
        Self {
            transactions: RefCell::new(transactions),
        }
    }
}

pub struct InMemoryStarknetTransactionManager {
    nfts: RefCell<HashMap<String, HashMap<String, String>>>,
}

impl StarknetManager for InMemoryStarknetTransactionManager {
    fn project_has_token(&self, project_id: &str, token_id: &str) -> bool {
        let nfts = self.nfts.borrow();
        nfts.contains_key(project_id) && nfts[project_id].contains_key(token_id)
    }

    fn mint_project_token(
        &self,
        project_id: &str,
        token_id: &str,
        starknet_account_addr: &str,
    ) -> Result<String, crate::domain::MintError> {
        let mut nfts = self.nfts.borrow_mut();
        if !nfts.contains_key(project_id) {
            nfts.insert(project_id.to_string(), HashMap::new());
        }

        nfts.get_mut(project_id)
            .unwrap()
            .insert(token_id.into(), starknet_account_addr.into());

        Ok(token_id.into())
    }
}

impl InMemoryStarknetTransactionManager {
    pub fn new() -> Self {
        Self {
            nfts: RefCell::new(HashMap::new()),
        }
    }
}
