use std::{cell::RefCell, ptr::read, sync::Arc};

use bridge_juno_to_starknet_backend::domain::{
    handle_bridge_request, BridgeError, BridgeRequest, BridgeResponse, MsgTypes::TransferNft,
    SignedHashValidator, SignedHashValidatorError, Transaction, TransactionFetchError,
    TransactionRepository,
};
use cucumber::{gherkin::Step, given, then, when, World};
use std::future::ready;

#[derive(Debug, World)]
struct BridgeWorld {
    request: Option<BridgeRequest>,
    response: Option<BridgeResponse>,
    validator: Option<Arc<dyn SignedHashValidator>>,
    transactions_repository: Option<Arc<dyn TransactionRepository>>,
}
impl BridgeWorld {
    fn with_signed_hash_validator(&mut self, validator: Arc<dyn SignedHashValidator>) {
        self.validator = Some(validator);
    }
    fn with_transaction_repository(&mut self, repository: Arc<dyn TransactionRepository>) {
        self.transactions_repository = Some(repository);
    }
}

impl Default for BridgeWorld {
    fn default() -> Self {
        Self {
            request: None,
            response: None,
            validator: None,
            transactions_repository: None,
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
            &row[0],
            &row[1],
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
fn when_i_execute_the_request(case: &mut BridgeWorld) {
    if let Some(request) = &case.request {
        case.response = Some(handle_bridge_request(
            request,
            "admin-account",
            case.validator.as_ref().unwrap().clone(),
            case.transactions_repository.as_ref().unwrap().clone(),
        ))
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
        let err = match response {
            Err(err) => err,
            Ok(_o) => panic!("Keplr wallet is incorrect please check implementation"),
        };
    };
}

#[then("I sould receive an error because current owner is not admin wallet")]
fn then_current_owner_is_not_admin(case: &mut BridgeWorld) {
    if let Some(response) = &case.response {
        let err = match response {
            Err(err) => err,
            Ok(_o) => panic!("Keplr wallet is incorrect please check implementation"),
        };
    };
}

#[derive(Debug, Clone)]
struct TestSignedHashValidator {}

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
struct InMemoryTransactionRepository {
    transactions: RefCell<Vec<Transaction>>,
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
                    TransferNft(tt) => tt,
                };
                t.contract == project_id && token_id == transfert.token_id
            })
            .collect::<Vec<Transaction>>();
        Ok(filtered_transactions)
    }
}

impl InMemoryTransactionRepository {
    fn new(transactions: Vec<Transaction>) -> Self {
        Self {
            transactions: RefCell::new(transactions),
        }
    }
}

fn main() {
    let validator = Arc::new(TestSignedHashValidator {});
    let world = BridgeWorld::cucumber().before(move |_feature, _rule, _scenario, _world| {
        _world.with_signed_hash_validator(validator.clone());
        Box::pin(ready(()))
    });

    futures::executor::block_on(world.run_and_exit("features/bridge.feature"));
}
