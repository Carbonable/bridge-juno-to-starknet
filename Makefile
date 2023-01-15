test:
	cargo test

run_api:
	cargo run --bin api -- --juno-lcd https://lcd-juno.itastakers.com --database-url postgresql://root:root@localhost:5432/starknet_bridge --juno-admin-address ${JUNO_ADMIN_ADDRESS} --starknet-admin-address ${STARKNET_ADMIN_ADDRESS} --starknet-network-id testnet-1 --starknet-admin-private-key ${STARKNET_ADMIN_PRIVATE_KEY} --frontend-uri http://localhost:3000 --batch-size 1

run_worker:
	cargo run --bin worker -- --juno-lcd https://lcd-juno.itastakers.com --database-url postgresql://root:root@localhost:5432/starknet_bridge --juno-admin-address ${JUNO_ADMIN_ADDRESS} --starknet-admin-address ${STARKNET_ADMIN_ADDRESS} --starknet-network-id testnet-1 --starknet-admin-private-key ${STARKNET_ADMIN_PRIVATE_KEY} --frontend-uri http://localhost:3000 --batch-size 1

deploy_preprod:
	fly deploy --config ./fly.preprod.toml

deploy_prod:
	fly deploy --config ./fly.prod.toml
