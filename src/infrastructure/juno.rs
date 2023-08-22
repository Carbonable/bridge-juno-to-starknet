use async_trait::async_trait;
use log::error;
use reqwest::Response;
use serde_derive::{Deserialize, Serialize};
use std::thread::sleep;
use std::time::Duration;

use crate::domain::bridge::{MsgTypes, Transaction, TransactionFetchError, TransactionRepository};

const MAX_RETRY: i32 = 5;

#[derive(Debug)]
pub enum JunoLcdError {
    ApiGetFailure(String),
    Reqwest(String),
}

pub struct JunoLcd {
    lcd_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionItem {
    body: Body,
    signatures: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Body {
    messages: Vec<Transaction>,
    memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TransactionResponseItem {
    messages: Vec<Transaction>,
    memo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionResponse {
    height: String,
    txhash: String,
    codespace: String,
    code: u64,
    data: String,
    raw_log: String,
    info: String,
    gas_wanted: String,
    gas_used: String,
    timestamp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionApiResponse {
    txs: Vec<TransactionItem>,
    #[serde(skip)]
    tx_responses: Vec<TransactionResponse>,
    pagination: Option<String>,
    total: String,
}

#[async_trait]
impl TransactionRepository for JunoLcd {
    async fn get_transactions_for_contract(
        &self,
        project_id: &str,
        token_id: &str,
    ) -> Result<Vec<crate::domain::bridge::Transaction>, crate::domain::bridge::TransactionFetchError>
    {
        // Hard limitting limit and offset as this is not relevant here to use it as a param.
        let endpoint = format!(
            "/cosmos/tx/v1beta1/txs?events=execute._contract_address=%27{}%27&pagination.limit=100&pagination.offset=0&pagination.count_total=true&reverse=true",
            project_id
        );
        let response = match self.get(endpoint).await {
            Ok(t) => t,
            Err(e) => {
                error!("fetching Juno blockchain transactions : {:#?}", e);
                return Err(TransactionFetchError::FetchError(
                    "Failed to call transaction API".into(),
                ));
            }
        };
        if 500 <= response.status().as_u16() {
            return Err(TransactionFetchError::JunoBlockchainServerError(
                response.status().into(),
            ));
        }

        let txs = match response.json::<TransactionApiResponse>().await {
            Ok(t) => t,
            Err(_e) => return Err(TransactionFetchError::DeserializationFailed),
        };

        let mut domain_tx: Vec<Transaction> = Vec::new();
        for transaction_item in txs.txs.iter() {
            for msg in transaction_item.body.messages.iter() {
                let transfer = match &msg.msg {
                    MsgTypes::TransferNft(t) => t,
                };

                if transfer.token_id == token_id {
                    domain_tx.push(msg.clone());
                }
            }
        }

        Ok(domain_tx)
    }
}

impl JunoLcd {
    pub fn new(lcd_address: &str) -> Self {
        Self {
            lcd_address: lcd_address.into(),
        }
    }

    async fn get(&self, endpoint: String) -> Result<Response, JunoLcdError> {
        for i in 0..MAX_RETRY {
            let addr = self.lcd_address.clone();
            if let Ok(client) = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
            {
                let request = client
                    .get(format!("{}{}", addr, endpoint.clone()))
                    .send()
                    .await;

                if request.is_err() {
                    if i < MAX_RETRY {
                        sleep(Duration::from_secs(15));
                        continue;
                    }
                    return Err(JunoLcdError::ApiGetFailure(endpoint));
                }

                return Ok(request.unwrap());
            } else {
                return Err(JunoLcdError::Reqwest("Failed to build client".into()));
            }
        }

        // Add notification here.
        Err(JunoLcdError::ApiGetFailure(endpoint))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::TransactionApiResponse;

    #[test]
    fn test_juno_deserialize_response() {
        let raw_str = r#"
{
	"txs": [
		{
			"body": {
				"messages": [
					{
						"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
						"sender": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
						"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
						"msg": {
							"transfer_nft": {
								"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
								"token_id": "232"
							}
						},
						"funds": []
					}
				],
				"memo": "transferNft",
				"timeout_height": "0",
				"extension_options": [],
				"non_critical_extension_options": []
			},
			"auth_info": {
				"signer_infos": [
					{
						"public_key": {
							"@type": "/cosmos.crypto.secp256k1.PubKey",
							"key": "A3cxHdNiq4Dbaxck4zNUMFxLI84qhrUQ5r/yxUansEua"
						},
						"mode_info": {
							"single": {
								"mode": "SIGN_MODE_DIRECT"
							}
						},
						"sequence": "3"
					}
				],
				"fee": {
					"amount": [
						{
							"denom": "ujuno",
							"amount": "17119"
						}
					],
					"gas_limit": "228244",
					"payer": "",
					"granter": ""
				},
				"tip": null
			},
			"signatures": [
				"KKQYCWidAEygT9qPIo0jtSLaDjg+PjW5yRZiZqWzxPcLcMXoHJLD275AwdzYKPpIa6gNFJ74SPD5vJfJd264iw=="
			]
		},
		{
			"body": {
				"messages": [
					{
						"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
						"sender": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
						"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
						"msg": {
							"transfer_nft": {
								"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
								"token_id": "111"
							}
						},
						"funds": []
					},
					{
						"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
						"sender": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
						"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
						"msg": {
							"transfer_nft": {
								"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
								"token_id": "112"
							}
						},
						"funds": []
					}
				],
				"memo": "transferNft",
				"timeout_height": "0",
				"extension_options": [],
				"non_critical_extension_options": []
			},
			"auth_info": {
				"signer_infos": [
					{
						"public_key": {
							"@type": "/cosmos.crypto.secp256k1.PubKey",
							"key": "A1i6aG9LBeeSOSQppEVMGIbtpjxL5OYqp45fRR15OiJw"
						},
						"mode_info": {
							"single": {
								"mode": "SIGN_MODE_DIRECT"
							}
						},
						"sequence": "2"
					}
				],
				"fee": {
					"amount": [
						{
							"denom": "ujuno",
							"amount": "27773"
						}
					],
					"gas_limit": "370295",
					"payer": "",
					"granter": ""
				},
				"tip": null
			},
			"signatures": [
				"tMa5LWlbzp1RQ+nkdFb/CTPdqVNWLAvcQPALamu6yQBOSALSb2S84d5ytf9GEGEuogkERr4Tyr3D7/lCgnRJnQ=="
			]
		}
	],
	"tx_responses": [
		{
			"height": "9408062",
			"txhash": "39751FD94B1B1517E5E19A263BFFE1339F4DBF8653BE442431AB08FC2F9D0940",
			"codespace": "",
			"code": 0,
			"data": "0A260A242F636F736D7761736D2E7761736D2E76312E4D736745786563757465436F6E7472616374",
			"raw_log": "[{\"events\":[{\"type\":\"execute\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"}]},{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"/cosmwasm.wasm.v1.MsgExecuteContract\"},{\"key\":\"module\",\"value\":\"wasm\"},{\"key\":\"sender\",\"value\":\"juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d\"}]},{\"type\":\"wasm\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"},{\"key\":\"action\",\"value\":\"transfer_nft\"},{\"key\":\"sender\",\"value\":\"juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d\"},{\"key\":\"recipient\",\"value\":\"juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada\"},{\"key\":\"token_id\",\"value\":\"232\"}]}]}]",
			"logs": [
				{
					"msg_index": 0,
					"log": "",
					"events": [
						{
							"type": "execute",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								}
							]
						},
						{
							"type": "message",
							"attributes": [
								{
									"key": "action",
									"value": "/cosmwasm.wasm.v1.MsgExecuteContract"
								},
								{
									"key": "module",
									"value": "wasm"
								},
								{
									"key": "sender",
									"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d"
								}
							]
						},
						{
							"type": "wasm",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								},
								{
									"key": "action",
									"value": "transfer_nft"
								},
								{
									"key": "sender",
									"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d"
								},
								{
									"key": "recipient",
									"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada"
								},
								{
									"key": "token_id",
									"value": "232"
								}
							]
						}
					]
				}
			],
			"info": "",
			"gas_wanted": "228244",
			"gas_used": "189921",
			"tx": {
				"@type": "/cosmos.tx.v1beta1.Tx",
				"body": {
					"messages": [
						{
							"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"sender": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"msg": {
								"transfer_nft": {
									"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
									"token_id": "232"
								}
							},
							"funds": []
						}
					],
					"memo": "transferNft",
					"timeout_height": "0",
					"extension_options": [],
					"non_critical_extension_options": []
				},
				"auth_info": {
					"signer_infos": [
						{
							"public_key": {
								"@type": "/cosmos.crypto.secp256k1.PubKey",
								"key": "A3cxHdNiq4Dbaxck4zNUMFxLI84qhrUQ5r/yxUansEua"
							},
							"mode_info": {
								"single": {
									"mode": "SIGN_MODE_DIRECT"
								}
							},
							"sequence": "3"
						}
					],
					"fee": {
						"amount": [
							{
								"denom": "ujuno",
								"amount": "17119"
							}
						],
						"gas_limit": "228244",
						"payer": "",
						"granter": ""
					},
					"tip": null
				},
				"signatures": [
					"KKQYCWidAEygT9qPIo0jtSLaDjg+PjW5yRZiZqWzxPcLcMXoHJLD275AwdzYKPpIa6gNFJ74SPD5vJfJd264iw=="
				]
			},
			"timestamp": "2023-08-02T13:03:50Z",
			"events": [
				{
					"type": "coin_spent",
					"attributes": [
						{
							"key": "spender",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						},
						{
							"key": "amount",
							"value": "17119ujuno",
							"index": true
						}
					]
				},
				{
					"type": "coin_received",
					"attributes": [
						{
							"key": "receiver",
							"value": "juno17xpfvakm2amg962yls6f84z3kell8c5lxtqmvp",
							"index": true
						},
						{
							"key": "amount",
							"value": "17119ujuno",
							"index": true
						}
					]
				},
				{
					"type": "transfer",
					"attributes": [
						{
							"key": "recipient",
							"value": "juno17xpfvakm2amg962yls6f84z3kell8c5lxtqmvp",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						},
						{
							"key": "amount",
							"value": "17119ujuno",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "sender",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "fee",
							"value": "17119ujuno",
							"index": true
						},
						{
							"key": "fee_payer",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "acc_seq",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d/3",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "signature",
							"value": "KKQYCWidAEygT9qPIo0jtSLaDjg+PjW5yRZiZqWzxPcLcMXoHJLD275AwdzYKPpIa6gNFJ74SPD5vJfJd264iw==",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "action",
							"value": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "module",
							"value": "wasm",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						}
					]
				},
				{
					"type": "execute",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						}
					]
				},
				{
					"type": "wasm",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						},
						{
							"key": "action",
							"value": "transfer_nft",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno1qzxw9f6vrefte6ezqffpsvp2vu4far95fr4s6d",
							"index": true
						},
						{
							"key": "recipient",
							"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
							"index": true
						},
						{
							"key": "token_id",
							"value": "232",
							"index": true
						}
					]
				}
			]
		},
		{
			"height": "9681296",
			"txhash": "04304465609CB2ECEFDA90E0E3278FBD3BE1BCDDB7818C702ED29AD8F0AF53E3",
			"codespace": "",
			"code": 0,
			"data": "122E0A2C2F636F736D7761736D2E7761736D2E76312E4D736745786563757465436F6E7472616374526573706F6E7365122E0A2C2F636F736D7761736D2E7761736D2E76312E4D736745786563757465436F6E7472616374526573706F6E7365",
			"raw_log": "[{\"msg_index\":0,\"events\":[{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"/cosmwasm.wasm.v1.MsgExecuteContract\"},{\"key\":\"sender\",\"value\":\"juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x\"},{\"key\":\"module\",\"value\":\"wasm\"}]},{\"type\":\"execute\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"}]},{\"type\":\"wasm\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"},{\"key\":\"action\",\"value\":\"transfer_nft\"},{\"key\":\"sender\",\"value\":\"juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x\"},{\"key\":\"recipient\",\"value\":\"juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada\"},{\"key\":\"token_id\",\"value\":\"111\"}]}]},{\"msg_index\":1,\"events\":[{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"/cosmwasm.wasm.v1.MsgExecuteContract\"},{\"key\":\"sender\",\"value\":\"juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x\"},{\"key\":\"module\",\"value\":\"wasm\"}]},{\"type\":\"execute\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"}]},{\"type\":\"wasm\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn\"},{\"key\":\"action\",\"value\":\"transfer_nft\"},{\"key\":\"sender\",\"value\":\"juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x\"},{\"key\":\"recipient\",\"value\":\"juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada\"},{\"key\":\"token_id\",\"value\":\"112\"}]}]}]",
			"logs": [
				{
					"msg_index": 0,
					"log": "",
					"events": [
						{
							"type": "message",
							"attributes": [
								{
									"key": "action",
									"value": "/cosmwasm.wasm.v1.MsgExecuteContract"
								},
								{
									"key": "sender",
									"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x"
								},
								{
									"key": "module",
									"value": "wasm"
								}
							]
						},
						{
							"type": "execute",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								}
							]
						},
						{
							"type": "wasm",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								},
								{
									"key": "action",
									"value": "transfer_nft"
								},
								{
									"key": "sender",
									"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x"
								},
								{
									"key": "recipient",
									"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada"
								},
								{
									"key": "token_id",
									"value": "111"
								}
							]
						}
					]
				},
				{
					"msg_index": 1,
					"log": "",
					"events": [
						{
							"type": "message",
							"attributes": [
								{
									"key": "action",
									"value": "/cosmwasm.wasm.v1.MsgExecuteContract"
								},
								{
									"key": "sender",
									"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x"
								},
								{
									"key": "module",
									"value": "wasm"
								}
							]
						},
						{
							"type": "execute",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								}
							]
						},
						{
							"type": "wasm",
							"attributes": [
								{
									"key": "_contract_address",
									"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn"
								},
								{
									"key": "action",
									"value": "transfer_nft"
								},
								{
									"key": "sender",
									"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x"
								},
								{
									"key": "recipient",
									"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada"
								},
								{
									"key": "token_id",
									"value": "112"
								}
							]
						}
					]
				}
			],
			"info": "",
			"gas_wanted": "370295",
			"gas_used": "300564",
			"tx": {
				"@type": "/cosmos.tx.v1beta1.Tx",
				"body": {
					"messages": [
						{
							"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"sender": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"msg": {
								"transfer_nft": {
									"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
									"token_id": "111"
								}
							},
							"funds": []
						},
						{
							"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"sender": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"contract": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"msg": {
								"transfer_nft": {
									"recipient": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
									"token_id": "112"
								}
							},
							"funds": []
						}
					],
					"memo": "transferNft",
					"timeout_height": "0",
					"extension_options": [],
					"non_critical_extension_options": []
				},
				"auth_info": {
					"signer_infos": [
						{
							"public_key": {
								"@type": "/cosmos.crypto.secp256k1.PubKey",
								"key": "A1i6aG9LBeeSOSQppEVMGIbtpjxL5OYqp45fRR15OiJw"
							},
							"mode_info": {
								"single": {
									"mode": "SIGN_MODE_DIRECT"
								}
							},
							"sequence": "2"
						}
					],
					"fee": {
						"amount": [
							{
								"denom": "ujuno",
								"amount": "27773"
							}
						],
						"gas_limit": "370295",
						"payer": "",
						"granter": ""
					},
					"tip": null
				},
				"signatures": [
					"tMa5LWlbzp1RQ+nkdFb/CTPdqVNWLAvcQPALamu6yQBOSALSb2S84d5ytf9GEGEuogkERr4Tyr3D7/lCgnRJnQ=="
				]
			},
			"timestamp": "2023-08-21T14:53:32Z",
			"events": [
				{
					"type": "coin_spent",
					"attributes": [
						{
							"key": "spender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "amount",
							"value": "27773ujuno",
							"index": true
						}
					]
				},
				{
					"type": "coin_received",
					"attributes": [
						{
							"key": "receiver",
							"value": "juno17xpfvakm2amg962yls6f84z3kell8c5lxtqmvp",
							"index": true
						},
						{
							"key": "amount",
							"value": "27773ujuno",
							"index": true
						}
					]
				},
				{
					"type": "transfer",
					"attributes": [
						{
							"key": "recipient",
							"value": "juno17xpfvakm2amg962yls6f84z3kell8c5lxtqmvp",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "amount",
							"value": "27773ujuno",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "fee",
							"value": "27773ujuno",
							"index": true
						},
						{
							"key": "fee_payer",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "acc_seq",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x/2",
							"index": true
						}
					]
				},
				{
					"type": "tx",
					"attributes": [
						{
							"key": "signature",
							"value": "tMa5LWlbzp1RQ+nkdFb/CTPdqVNWLAvcQPALamu6yQBOSALSb2S84d5ytf9GEGEuogkERr4Tyr3D7/lCgnRJnQ==",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "action",
							"value": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "module",
							"value": "wasm",
							"index": true
						}
					]
				},
				{
					"type": "execute",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						}
					]
				},
				{
					"type": "wasm",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						},
						{
							"key": "action",
							"value": "transfer_nft",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "recipient",
							"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
							"index": true
						},
						{
							"key": "token_id",
							"value": "111",
							"index": true
						}
					]
				},
				{
					"type": "message",
					"attributes": [
						{
							"key": "action",
							"value": "/cosmwasm.wasm.v1.MsgExecuteContract",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "module",
							"value": "wasm",
							"index": true
						}
					]
				},
				{
					"type": "execute",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						}
					]
				},
				{
					"type": "wasm",
					"attributes": [
						{
							"key": "_contract_address",
							"value": "juno13g5r0tmmngmm9d0clwa7exjamxxxag5p5fgdra7qjtaexdg6yprq5298fn",
							"index": true
						},
						{
							"key": "action",
							"value": "transfer_nft",
							"index": true
						},
						{
							"key": "sender",
							"value": "juno17n26lald8gh3hpvzysrnv9zjkyysh0j66sw60x",
							"index": true
						},
						{
							"key": "recipient",
							"value": "juno1d8nt7t2tsnkzqk2qt94dt5nesfye6m4kfu2ada",
							"index": true
						},
						{
							"key": "token_id",
							"value": "112",
							"index": true
						}
					]
				}
			]
		}
	],
	"pagination": null,
	"total": "2"
}
            "#;

        let res: TransactionApiResponse = match serde_json::from_str(raw_str) {
            Ok(r) => r,
            Err(e) => panic!("{:#?}", e),
        };
    }
}
