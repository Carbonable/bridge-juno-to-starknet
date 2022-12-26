use std::{future::ready, sync::Arc};

use bridge_juno_to_starknet_backend::{
    domain::save_customer_data::{
        handle_save_customer_data, DataRepository, SaveCustomerDataRequest,
    },
    infrastructure::in_memory::InMemoryDataRepository,
};
use cucumber::{gherkin::Step, given, then, when, World};

#[derive(Debug, World)]
struct SaveCustomerDataWorld {
    request: Option<SaveCustomerDataRequest>,
    response: bool,
    data_repository: Option<Arc<dyn DataRepository>>,
}

impl SaveCustomerDataWorld {}
impl Default for SaveCustomerDataWorld {
    fn default() -> Self {
        Self {
            request: None,
            response: false,
            data_repository: None,
        }
    }
}

impl SaveCustomerDataWorld {
    fn with_data_repo(&mut self, repo: Arc<dyn DataRepository>) {
        self.data_repository = Some(repo);
    }
}

#[given("a request")]
fn given_a_request(case: &mut SaveCustomerDataWorld, step: &Step) {
    let Some(table) = step.table.as_ref() else { return };

    for row in table.rows.iter().skip(1) {
        // Retrieving col values with number.
        let request = SaveCustomerDataRequest::new(
            &row[0],
            &row[1],
            row[2]
                .replace("[", "")
                .replace("]", "")
                .split(", ")
                .collect::<Vec<&str>>(),
        );

        case.request = Some(request);
    }
}

#[when("I execute the request")]
async fn when_i_execute_the_request(case: &mut SaveCustomerDataWorld) {
    let response = handle_save_customer_data(
        case.request.as_ref().unwrap(),
        case.data_repository.as_ref().unwrap().clone(),
    )
    .await;

    if response.is_err() {
        panic!("Response has to be correct in here");
    }

    case.response = response.is_err();
}

#[then("data should have been persisted to database")]
async fn then_data_should_have_been_persited(case: &mut SaveCustomerDataWorld) {
    let repo = case.data_repository.as_ref().unwrap().clone();
    let req = case.request.as_ref().unwrap();

    let customer_keys = match repo
        .get_customer_keys(&req.keplr_wallet_pubkey, &req.project_id)
        .await
    {
        Ok(ck) => ck,
        Err(_) => panic!("Customer keys has not been persisted into database"),
    };
}

fn main() {
    let repo = Arc::new(InMemoryDataRepository::new());
    let world =
        SaveCustomerDataWorld::cucumber().before(move |_feature, _rule, _scenario, _world| {
            _world.with_data_repo(repo.clone());
            Box::pin(ready(()))
        });

    futures::executor::block_on(world.run_and_exit("features/save-customer-data.feature"));
}
