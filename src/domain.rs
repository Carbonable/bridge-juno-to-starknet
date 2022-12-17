#[derive(Debug)]
pub struct BridgeRequest {
    signed_hash: String,
    starknet_account_addrr: String,
    keplr_wallet_pubkey: String,
    project_id: String,
    tokens_id: Vec<String>,
}

impl BridgeRequest {
    pub fn new(
        signed_hash: &str,
        starknet_account_addrr: &str,
        keplr_wallet_pubkey: &str,
        project_id: &str,
        tokens_id: Vec<&str>,
    ) -> Self {
        let mut tokens = vec![];
        for t in tokens_id {
            tokens.push(t.into());
        }
        Self {
            signed_hash: signed_hash.into(),
            starknet_account_addrr: starknet_account_addrr.into(),
            keplr_wallet_pubkey: keplr_wallet_pubkey.into(),
            project_id: project_id.into(),
            tokens_id: tokens,
        }
    }
}
