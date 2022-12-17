use bridge_juno_to_starknet_backend::domain::BridgeRequest;
use cucumber::{gherkin::Step, given, World};

#[derive(Debug, Default, World)]
struct BridgeWorld {}

#[given("a request with values:")]
fn given_request_with_values(case: &mut BridgeWorld, step: &Step) {
    let Some(table) = step.table.as_ref() else { return };
    // Skipping first row as it is headers
    for row in table.rows.iter().skip(1) {
        // Retrieving col values with number.

        let tokens = vec!["token-1", "token-2"];
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
        println!("{:#?}", request);
    }
}

fn main() {
    futures::executor::block_on(BridgeWorld::run("features/bridge.feature"));
}
