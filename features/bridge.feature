Feature: Bridge between Juno and Starknet for carbonABLE NFT's
    Rule: 
        - Receive a signed hash, starknet wallet address, customer's keplr wallet public key, a list of token ids, project id.
        - Check the signed hash is correct
        - Check customers keplr wallet was the last owner of tokens
        - Check customer balance is effectively 0
        - Check admin address on Juno is now owner of tokens for project id
        - Enqueue the requested tokens 

    Scenario: Signed hash is incorrect
        Given the following transaction list
            """ []
            """
        Given a request with values:
            | signed_hash | starknet_account_addr | keplr_customer_pubkey | project_id | tokens_ids |
            | anInvalidHash | st4rkn3t-1 | k3plr-pk1 | projectId | [254, 255] |
        When I execute the request
        Then the signed hash should not be valid

    Scenario: Check keplr customer wallet was the last owner of tokens
        Given the following transaction list
            """
            [
                {
                    "sender": "sender-1",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "juno-admin-account",
                            "token_id": "255"
                        }
                    }
                },
                {
                    "sender": "carbonABLE",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "not-the-customer",
                            "token_id": "255"
                        }
                    }
                },
                {
                    "sender": "carbonABLE",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "k3plr-pk1",
                            "token_id": "254"
                        }
                    }
                },
                {
                    "sender": "carbonABLE",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "k3plr-pk1",
                            "token_id": "255"
                        }
                    }
                }
            ]
            """
        Given a request with values:
            | signed_hash | starknet_account_addr | keplr_customer_pubkey | project_id | tokens_ids |
            | aValidSignedHash | st4rkn3t-1 | k3plr-pk1 | projectId | [255] |
        When I execute the request
        Then I sould receive an error because provided keplr wallet was not the previous owner

    Scenario: Check last transaction is owned by admin-wallet
        Given the following transaction list
            """
            [
                {
                    "sender": "sender-1",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "not-juno-admin-account",
                            "token_id": "255"
                        }
                    }
                },
                {
                    "sender": "carbonABLE",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "not-the-customer",
                            "token_id": "255"
                        }
                    }
                }
            ]
            """
        Given a request with values:
            | signed_hash | starknet_account_addr | keplr_customer_pubkey | project_id | tokens_ids |
            | aValidSignedHash | st4rkn3t-1 | k3plr-pk1 | projectId | [255] |
        When I execute the request
        Then I sould receive an error because current owner is not admin wallet

    Scenario: Transaction are ok, last one has admin has recipient and current one has customers wallet
        Given the following transaction list
            """
            [
                {
                    "sender": "k3plr-pk1",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "juno-admin-account",
                            "token_id": "255"
                        }
                    }
                },
                {
                    "sender": "k3plr-pk1",
                    "contract": "projectId",
                    "msg": {
                        "transfer_nft": {
                            "recipient": "juno-admin-account",
                            "token_id": "254"
                        }
                    }
                }
            ]
            """
        Given a request with values:
            | signed_hash | starknet_account_addr | keplr_customer_pubkey | project_id | tokens_ids |
            | aValidSignedHash | st4rkn3t-1 | k3plr-pk1 | projectId | [254, 255] |
        When I execute the request
        Then nfts migration request should have been enqueued and response should be ok
