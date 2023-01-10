use crate::domain::save_customer_data::{CustomerKeys, DataRepository, SaveCustomerDataError};
use async_trait::async_trait;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use log::error;
use std::sync::Arc;
use tokio_postgres::{Config, Error, NoTls};

pub async fn get_connection(database_uri: &str) -> core::result::Result<Pool, Error> {
    let config = database_uri.parse::<Config>()?;
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Verified,
    };
    let manager = Manager::from_config(config, NoTls, manager_config);
    let pool = Pool::builder(manager).max_size(16).build().unwrap();

    Ok(pool)
}

pub struct PostgresDataRepository {
    connection_pool: Arc<Pool>,
}
impl PostgresDataRepository {
    pub fn new(connection_pool: Arc<Pool>) -> Self {
        Self { connection_pool }
    }
}

#[async_trait]
impl DataRepository for PostgresDataRepository {
    async fn save_customer_keys(&self, keys: CustomerKeys) -> Result<(), SaveCustomerDataError> {
        let client = self.connection_pool.clone().get().await.unwrap();

        let insert = client.execute(
            "INSERT INTO customer_keys (keplr_wallet_pubkey, project_id, token_ids) VALUES ($1, $2, $3)",
            &[&keys.keplr_wallet_pubkey, &keys.project_id, &keys.token_ids]
            ).await;
        if insert.is_err() {
            error!("Error while inserting customer to database {:#?}", insert);
            let update = client.execute(
                "UPDATE customer_keys SET token_ids = $1 WHERE keplr_wallet_pubkey = $2 AND project_id = $3",
                &[&keys.token_ids, &keys.keplr_wallet_pubkey, &keys.project_id]).await;

            if update.is_err() {
                error!("Error while saving customer to database {:#?}", update);
                return Err(SaveCustomerDataError::FailedToPersistToDatabase);
            }

            return Ok(());
        }

        if 1 == insert.unwrap() {
            return Ok(());
        }

        Err(SaveCustomerDataError::NotImpled)
    }

    async fn get_customer_keys(
        &self,
        keplr_wallet_pubkey: &str,
        project_id: &str,
    ) -> Result<CustomerKeys, SaveCustomerDataError> {
        let client = self.connection_pool.clone().get().await.unwrap();

        let query = client.prepare("SELECT * FROM customer_keys ck WHERE ck.keplr_wallet_pubkey = $1 AND ck.project_id = $2").await.unwrap();

        let rows = match client
            .query(&query, &[&keplr_wallet_pubkey, &project_id])
            .await
        {
            Ok(r) => r,
            Err(_e) => return Err(SaveCustomerDataError::NotFound),
        };
        if 0 == rows.len() {
            return Err(SaveCustomerDataError::NotFound);
        }
        let row = &rows[0];
        let customer_keys = CustomerKeys {
            keplr_wallet_pubkey: row.get::<usize, String>(1).into(),
            project_id: row.get::<usize, String>(2).into(),
            token_ids: row.get::<usize, Vec<String>>(3).into(),
        };

        Ok(customer_keys)
    }
}
