use std::{ptr::read, sync::Arc};

use bridge_juno_to_starknet_backend::domain::{
    handle_bridge_request, BridgeRequest, BridgeResponse, SignedHashValidator,
    SignedHashValidatorError,
};
use cucumber::{gherkin::Step, given, then, when, World};
use std::future::ready;

#[derive(Debug, World)]
struct BridgeWorld {
    request: Option<BridgeRequest>,
    response: Option<BridgeResponse>,
    validator: Option<Arc<dyn SignedHashValidator>>,
}
impl BridgeWorld {
    fn with_signed_hash_validator(&mut self, validator: Arc<dyn SignedHashValidator>) {
        self.validator = Some(validator);
    }
}

impl Default for BridgeWorld {
    fn default() -> Self {
        Self {
            request: None,
            response: None,
            validator: None,
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

#[when("I execute the request")]
fn when_i_execute_the_request(case: &mut BridgeWorld) {
    if let Some(request) = &case.request {
        case.response = Some(handle_bridge_request(
            request,
            case.validator.as_ref().unwrap().clone(),
        ))
    }
}

#[then("the signed hash should not be valid")]
fn then_the_signed_hash_sould_not_be_valid(case: &mut BridgeWorld) {
    if let Some(response) = &case.response {
        if response.is_ok() {
            panic!("Signed hash sould not be valid. Please check for implementation")
        }
    }
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

fn main() {
    let validator = Arc::new(TestSignedHashValidator {});
    let world = BridgeWorld::cucumber().before(move |_feature, _rule, _scenario, _world| {
        _world.with_signed_hash_validator(validator.clone());
        Box::pin(ready(()))
    });

    futures::executor::block_on(world.run_and_exit("features/bridge.feature"));
}
