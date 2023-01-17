use super::bridge::{QueueItem, QueueManager, StarknetManager};
use log::{error, info};
use std::{collections::HashMap, sync::Arc};

pub enum ConsumerError {
    FailedToGetNextBatch,
}
pub async fn consume_queue(
    queue_manager: Arc<dyn QueueManager>,
    starknet_manager: Arc<dyn StarknetManager>,
) -> Result<(), ConsumerError> {
    let batch = match queue_manager.get_batch().await {
        Ok(b) => b,
        Err(_e) => return Err(ConsumerError::FailedToGetNextBatch),
    };

    let mut token_to_mint: HashMap<String, Vec<QueueItem>> = HashMap::new();
    for qi in batch {
        if starknet_manager
            .project_has_token(&qi.project_id, &qi.token_id.as_str())
            .await
        {
            error!("Token id {} has already been minted", &qi.token_id);
            continue;
        }

        let project_id = qi.project_id.clone();
        match token_to_mint.entry(project_id.to_string()) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(vec![qi.clone()]);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                e.get_mut().push(qi.clone());
            }
        };
    }

    if 0 == token_to_mint.len() {
        info!("No token have been minted during this batch");
        return Ok(());
    }

    for (project_id, qi) in token_to_mint.iter() {
        let ids = qi
            .iter()
            .map(|q| q.id.as_ref().unwrap().to_string())
            .collect();

        queue_manager
            .update_queue_items_status(
                &ids,
                String::from(""),
                super::bridge::QueueStatus::Processing,
            )
            .await;

        let _mint = match starknet_manager
            .batch_mint_tokens(project_id, qi.to_vec())
            .await
        {
            Ok((tx_hash, status)) => {
                info!("Transaction {:#?} was handled successfully", tx_hash);
                let res = queue_manager
                    .update_queue_items_status(&ids, tx_hash, status)
                    .await;
                match res {
                    Ok(_r) => {
                        info!("Successfully updated queue item statuses");
                    }
                    Err(e) => {
                        error!("Error while update queue items status {:#?}", e);
                    }
                }
            }
            Err(_e) => {
                error!("Failed to create transaction");
            }
        };
    }

    Ok(())
}
