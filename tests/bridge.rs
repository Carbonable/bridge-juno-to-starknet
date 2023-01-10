use std::sync::Arc;

use bridge_juno_to_starknet_backend::{
    domain::{
        bridge::{
            handle_bridge_request, BridgeError, BridgeRequest, BridgeResponse, SignedHash,
            SignedHashValidator, StarknetManager, Transaction, TransactionRepository,
        },
        save_customer_data::DataRepository,
    },
    infrastructure::in_memory::{
        InMemoryDataRepository, InMemoryStarknetTransactionManager, InMemoryTransactionRepository,
        TestSignedHashValidator,
    },
};
use cucumber::{gherkin::Step, given, then, when, World};
use std::future::ready;

const STARKNET_PROJECT_ADDR: &str = "starknet_project_addr";

#[derive(Debug, World)]
struct BridgeWorld {
    request: Option<BridgeRequest>,
    response: Option<Result<BridgeResponse, BridgeError>>,
    validator: Option<Arc<dyn SignedHashValidator>>,
    transactions_repository: Option<Arc<dyn TransactionRepository>>,
    starknet_manager: Option<Arc<dyn StarknetManager>>,
    data_repository: Option<Arc<dyn DataRepository>>,
}
impl BridgeWorld {
    fn with_signed_hash_validator(&mut self, validator: Arc<dyn SignedHashValidator>) {
        self.validator = Some(validator);
    }
    fn with_transaction_repository(&mut self, repository: Arc<dyn TransactionRepository>) {
        self.transactions_repository = Some(repository);
    }
    fn with_starknet_manager(&mut self, manager: Arc<dyn StarknetManager>) {
        self.starknet_manager = Some(manager);
    }
    fn with_data_repository(&mut self, data_repository: Arc<dyn DataRepository>) {
        self.data_repository = Some(data_repository);
    }
}

impl Default for BridgeWorld {
    fn default() -> Self {
        Self {
            request: None,
            response: None,
            validator: None,
            transactions_repository: None,
            starknet_manager: None,
            data_repository: None,
        }
    }
}

#[given("a request with values:")]
fn given_request_with_values(case: &mut BridgeWorld, step: &Step) {
    let Some(table) = step.table.as_ref() else { return };
    // Skipping first row as it is headers
    for row in table.rows.iter().skip(1) {
        // Retrieving col values with number.
        let request = BridgeRequest::new(
            SignedHash {
                pub_key: bridge_juno_to_starknet_backend::domain::bridge::PubKey {
                    key_type: "tendermint/PubKeySecp256k1".into(),
                    key_value: "Avt8e5UqfoRAh0RBUzHCu9arv7UFEFdfcv657h6TtSZE".into(),
                },
                signature: row[0].to_string(),
            },
            &row[1],
            STARKNET_PROJECT_ADDR,
            &row[2],
            &row[3],
            row[4]
                .replace("[", "")
                .replace("]", "")
                .split(", ")
                .collect::<Vec<&str>>(),
        );

        case.request = Some(request);
    }
}

#[given("the following transaction list")]
fn given_the_following_transactions_list(case: &mut BridgeWorld, step: &Step) {
    let transactions: Vec<Transaction> =
        serde_json::from_str(step.docstring.as_ref().unwrap()).unwrap();
    let transaction_repository = Arc::new(InMemoryTransactionRepository::new(transactions));
    case.with_transaction_repository(transaction_repository);
}

#[when("I execute the request")]
async fn when_i_execute_the_request(case: &mut BridgeWorld) {
    if let Some(request) = &case.request {
        case.response = Some(
            handle_bridge_request(
                request,
                "juno-admin-account",
                "starknet-admin-account",
                case.validator.as_ref().unwrap().clone(),
                case.transactions_repository.as_ref().unwrap().clone(),
                case.starknet_manager.as_ref().unwrap().clone(),
                case.data_repository.as_ref().unwrap().clone(),
            )
            .await,
        )
    }
}

#[then("the signed hash should not be valid")]
fn then_the_signed_hash_sould_not_be_valid(case: &mut BridgeWorld) {
    if let Some(response) = &case.response {
        if response.is_ok() {
            panic!("Signed hash sould not be valid. Please check implementation")
        }
    }
}

#[then("I sould receive an error because provided keplr wallet was not the previous owner")]
fn then_keplr_provided_wallet_incorrect(case: &mut BridgeWorld) {
    if let Some(response) = &case.response {
        let _err = match response {
            Err(err) => err,
            Ok(res) => {
                if res.checks.contains_key("255") {
                    if let Some((_token, err)) = res.checks.get("255") {
                        if err.is_none() {
                            panic!("Provided keplr wallet should not be correct, please check implementation");
                        }
                    } else {
                        panic!("Should not fall into this case");
                    }
                }
                &BridgeError::InvalidSign
            }
        };
    };
}

#[then("I sould receive an error because current owner is not admin wallet")]
fn then_current_owner_is_not_admin(case: &mut BridgeWorld) {
    if let Some(response) = &case.response {
        let _err = match response {
            Err(err) => err,
            Ok(res) => {
                if res.checks.contains_key("255") {
                    if let Some((_token, err)) = res.checks.get("255") {
                        if err.is_none() {
                            panic!("Provided keplr wallet should not be correct, please check implementation");
                        }
                    } else {
                        panic!("Should not fall into this case");
                    }
                }
                &BridgeError::InvalidSign
            }
        };
    };
}

#[then("nfts token should be minted on starknet and response sould be ok")]
async fn then_nfts_should_be_minted_on_starknet(case: &mut BridgeWorld) {
    let starknet_project_id = &case.request.as_ref().unwrap().starknet_project_addr;
    let tokens_id = &case.request.as_ref().unwrap().tokens_id;
    let starknet_manager = case.starknet_manager.as_ref().unwrap().clone();
    if let Some(response) = &case.response {
        let _r = match response {
            Err(err) => panic!("{:#?}", err),
            Ok(r) => r,
        };
        for token in tokens_id.as_ref().unwrap() {
            if !starknet_manager
                .project_has_token(starknet_project_id, token)
                .await
            {
                panic!("Token {:#?} has not been minted on starknet", token)
            }
        }
    }
}

fn main() {
    let validator = Arc::new(TestSignedHashValidator {});
    let starknet_manager = Arc::new(InMemoryStarknetTransactionManager::new());
    let data_repository = Arc::new(InMemoryDataRepository::new());
    let world = BridgeWorld::cucumber().before(move |_feature, _rule, _scenario, _world| {
        _world.with_signed_hash_validator(validator.clone());
        _world.with_starknet_manager(starknet_manager.clone());
        _world.with_data_repository(data_repository.clone());
        Box::pin(ready(()))
    });

    futures::executor::block_on(world.run_and_exit("features/bridge.feature"));
}
