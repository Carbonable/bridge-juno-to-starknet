Bridge from Juno to Starknet for carbonABLE tokens
===

Developpment
---
Run project locally
```shell
JUNO_ADMIN_ADDRESS=changeme STARKNET_ADMIN_ADDRESS=changeme STARKNET_ADMIN_PRIVATE_KEY=changeme make run
```

Run integration tests:
```shell
make test 
```
Alternatively :
```shell
cargo test
```

Deployment
---
Make sure you have [flyctl](https://fly.io/docs/hands-on/install-flyctl/) installed.

To deploy **preprod** env:
```shell
make deploy_preprod
```
To deploy **prod** env:
```shell
make deploy_prod
```
