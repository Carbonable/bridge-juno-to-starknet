use crate::domain::save_customer_data::{CustomerKeys, DataRepository, SaveCustomerDataError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio_postgres::{Client, Error, NoTls};

pub async fn get_connection(database_uri: &str) -> core::result::Result<Client, Error> {
    let (client, connection) = tokio_postgres::connect(database_uri, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}

pub struct PostgresDataRepository {
    connection: Arc<Client>,
}
impl PostgresDataRepository {
    pub fn new(connection: Arc<Client>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl DataRepository for PostgresDataRepository {
    async fn save_customer_keys(&self, keys: CustomerKeys) -> Result<(), SaveCustomerDataError> {
        let insert = self.connection.clone().execute(
            "INSERT INTO customer_keys (keplr_wallet_pubkey, project_id, token_ids) VALUES ($1, $2, $3)",
            &[&keys.keplr_wallet_pubkey, &keys.project_id, &keys.token_ids]
            ).await;
        if insert.is_err() {
            let update = self.connection.clone().execute(
                "UPDATE customer_keys SET token_ids = $1 WHERE keplr_wallet_pubkey = $2 AND project_id = $3",
                &[&keys.token_ids, &keys.keplr_wallet_pubkey, &keys.project_id]).await;

            if update.is_err() {
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
        let client = self.connection.clone();
        let query = client.prepare("SELECT * FROM customer_keys ck WHERE ck.keplr_wallet_pubkey = $1 AND ck.project_id = $2").await.unwrap();

        let rows = match client
            .query(&query, &[&keplr_wallet_pubkey, &project_id])
            .await
        {
            Ok(r) => r,
            Err(_e) => return Err(SaveCustomerDataError::NotFound),
        };
        let row = &rows[0];
        let customer_keys = CustomerKeys {
            keplr_wallet_pubkey: row.get::<usize, String>(1).into(),
            project_id: row.get::<usize, String>(2).into(),
            token_ids: row.get::<usize, Vec<String>>(3).into(),
        };

        Ok(customer_keys)
    }
}
