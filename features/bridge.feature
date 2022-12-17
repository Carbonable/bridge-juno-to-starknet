Feature: Bridge between Juno and Starknet for carbonABLE NFT's
    Rule: 
        - Receive a signed hash, starknet wallet address, customer's keplr wallet public key, a list of token ids, project id.
        - Check the signed hash is correct
        - Check customers keplr wallet was the last owner of tokens
        - Check customer balance is effectively 0
        - Check admin address on Juno is now owner of tokens for project id
        - Mint tokens on starknet.

    Scenario: Signed hash is incorrect
        Given a request with values:
            | signed_hash | starknet_account_addr | keplr_customer_pubkey | project_id | tokens_ids |
            | aVerySignedHas | st4rkn3t-1 | k3plr-pk1 | projectId | [token1, token2] |
