use async_trait::async_trait;
use std::{collections::HashMap, sync::Mutex};

use crate::domain::{
    bridge::{
        MintError, MsgTypes, QueueError, QueueItem, QueueManager, QueueStatus, QueueUpdateError,
        SignedHash, SignedHashValidator, SignedHashValidatorError, StarknetManager, Transaction,
        TransactionFetchError, TransactionRepository,
    },
    save_customer_data::{CustomerKeys, DataRepository, SaveCustomerDataError},
};

#[derive(Debug, Clone)]
pub struct TestSignedHashValidator {}

impl SignedHashValidator for TestSignedHashValidator {
    fn verify(
        &self,
        signed_hash: &SignedHash,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
    ) -> Result<String, SignedHashValidatorError> {
        return match signed_hash.signature.as_str() {
            "anInvalidHash" => Err(SignedHashValidatorError::FailedToVerifyHash),
            &_ => Ok(signed_hash.signature.to_string()),
        };
    }
}

#[derive(Debug)]
pub struct InMemoryTransactionRepository {
    pub transactions: Mutex<Vec<Transaction>>,
}

#[async_trait]
impl TransactionRepository for InMemoryTransactionRepository {
    async fn get_transactions_for_contract(
        &self,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<Transaction>, TransactionFetchError> {
        let lock = match self.transactions.lock() {
            Ok(l) => l,
            _ => {
                return Err(TransactionFetchError::FetchError(
                    "Failed to acquire lock on the requested resource".into(),
                ))
            }
        };
        let filtered_transactions: Vec<Transaction> = lock
            .clone()
            .into_iter()
            .filter(|t| {
                let transfert = match &t.msg {
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
            transactions: Mutex::new(transactions),
        }
    }
}

pub struct InMemoryStarknetTransactionManager {
    nfts: Mutex<HashMap<String, HashMap<String, String>>>,
}

#[async_trait]
impl StarknetManager for InMemoryStarknetTransactionManager {
    async fn project_has_token(&self, project_id: &str, token_id: &str) -> bool {
        let lock = match self.nfts.lock() {
            Ok(l) => l,
            _ => {
                return false;
            }
        };

        lock.contains_key(project_id) && lock[project_id].contains_key(token_id)
    }

    async fn mint_project_token(
        &self,
        project_id: &str,
        tokens: &[String],
        starknet_account_addr: &str,
    ) -> Result<String, crate::domain::bridge::MintError> {
        let mut lock = match self.nfts.lock() {
            Ok(l) => l,
            _ => return Err(MintError::Failure),
        };

        if lock.contains_key(project_id) {
            lock.insert(project_id.to_string(), HashMap::new());
        }

        for token_id in tokens {
            lock.get_mut(project_id)
                .unwrap()
                .insert(token_id.to_string(), starknet_account_addr.into());
        }

        Ok("0xHExaD3c1m4lTr4ns4ct10nH4sH".to_string())
    }

    async fn batch_mint_tokens(
        &self,
        project_id: &str,
        queue_items: Vec<QueueItem>,
    ) -> Result<(String, QueueStatus), MintError> {
        Ok((
            "0xHExaD3c1m4lTr4ns4ct10nH4sH".to_string(),
            QueueStatus::Success,
        ))
    }
}

impl InMemoryStarknetTransactionManager {
    pub fn new() -> Self {
        Self {
            nfts: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug)]
pub struct InMemoryDataRepository {
    data: Mutex<HashMap<String, HashMap<String, Vec<String>>>>,
}

impl InMemoryDataRepository {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}
#[async_trait]
impl DataRepository for InMemoryDataRepository {
    async fn save_customer_keys(&self, keys: CustomerKeys) -> Result<(), SaveCustomerDataError> {
        let mut lock = match self.data.lock() {
            Ok(l) => l,
            Err(_) => panic!("Failed to acquire lock on data repository"),
        };

        if !lock.contains_key(&keys.keplr_wallet_pubkey) {
            let mut content: HashMap<String, Vec<String>> = HashMap::new();
            content.insert(keys.project_id.into(), keys.token_ids);
            lock.insert(keys.keplr_wallet_pubkey.into(), content);
            return Ok(());
        }
        if !lock[&keys.keplr_wallet_pubkey].contains_key(&keys.project_id) {
            lock.get_mut(&keys.keplr_wallet_pubkey)
                .expect("Failed to get data for customer keplr wallet")
                .insert(keys.project_id.into(), keys.token_ids);
            return Ok(());
        }

        let tokens = lock
            .get_mut(&keys.keplr_wallet_pubkey)
            .expect("Failed to get data for customer keplr wallet")
            .get_mut(&keys.project_id)
            .expect("Failed to get data from customer keplr wallet for project");
        for t in &keys.token_ids {
            tokens.push(t.into());
        }

        Ok(())
    }

    async fn get_customer_keys(
        &self,
        keplr_wallet_pubkey: &str,
        project_id: &str,
    ) -> Result<CustomerKeys, SaveCustomerDataError> {
        let lock = match self.data.lock() {
            Ok(l) => l,
            Err(e) => panic!("Failed to acquire lock on data repository: {:#?}", e),
        };

        if !lock.contains_key(keplr_wallet_pubkey)
            || !lock
                .get(keplr_wallet_pubkey)
                .unwrap()
                .contains_key(project_id)
        {
            return Err(SaveCustomerDataError::NotFound);
        }

        let tokens = lock
            .get(keplr_wallet_pubkey)
            .unwrap()
            .get(project_id)
            .unwrap();

        Ok(CustomerKeys {
            keplr_wallet_pubkey: keplr_wallet_pubkey.into(),
            project_id: project_id.into(),
            token_ids: tokens.to_vec(),
        })
    }
}

pub struct InMemoryQueueManager {
    pub queue: Mutex<HashMap<String, QueueItem>>,
}

impl InMemoryQueueManager {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(HashMap::new()),
        }
    }

    fn get_queue_identifier(pubkey: &str, project_id: &str, token: &str) -> String {
        format!("{pubkey}//{project_id}//{token}")
    }
}

#[async_trait]
impl QueueManager for InMemoryQueueManager {
    async fn enqueue(
        &self,
        keplr_wallet_pubkey: &str,
        starknet_wallet_pubkey: &str,
        project_id: &str,
        token_ids: Vec<String>,
    ) -> Result<Vec<QueueItem>, QueueError> {
        let mut lock = match self.queue.lock() {
            Ok(l) => l,
            Err(_) => panic!("Failed to acquire lock on queue"),
        };

        let mut inserted_queue_items = Vec::new();
        for token in token_ids {
            let qi = QueueItem::new(
                keplr_wallet_pubkey,
                starknet_wallet_pubkey,
                project_id,
                token.to_string(),
            );
            lock.insert(
                Self::get_queue_identifier(keplr_wallet_pubkey, project_id, token.as_str()),
                qi.clone(),
            );
            inserted_queue_items.push(qi.clone());
        }

        Ok(inserted_queue_items)
    }

    async fn get_batch(&self) -> Result<Vec<QueueItem>, QueueError> {
        let lock = match self.queue.lock() {
            Ok(l) => l,
            Err(_) => panic!("Failed to get lock on batch"),
        };

        let mut queue_items = Vec::new();
        for (_keplr_pubkey, qi) in lock.iter() {
            queue_items.push(qi.clone());
        }

        Ok(queue_items)
    }
    async fn get_customer_migration_state(
        &self,
        project_id: &str,
        keplr_wallet_pubkey: &str,
    ) -> Vec<QueueItem> {
        let lock = match self.queue.lock() {
            Ok(l) => l,
            Err(_) => panic!("Failed to acquire lock on resource"),
        };

        let mut queue_items = Vec::new();
        for (id, qi) in lock.iter() {
            if id
                .as_str()
                .starts_with(format!("{keplr_wallet_pubkey}//{project_id}").as_str())
            {
                continue;
            }

            queue_items.push(qi.clone());
        }

        queue_items
    }

    async fn update_queue_items_status(
        &self,
        ids: &Vec<String>,
        transaction_hash: String,
        status: QueueStatus,
    ) -> Result<(), QueueUpdateError> {
        Ok(())
    }
}
