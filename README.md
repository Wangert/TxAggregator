# TxAggregator

 Cross-chain transaction aggregator.

## Getting Started

#### Run the TxAggregator

```shell
cd ./main/src/main
cargo run CTXA
```

#### Use CLI

**register CosmosIBC chain toml file**

```shell
chain register -c "file's address"
```

**create client**

```shell
client create --source <sourece_chain_name> --target <target_chain_name> --clienttype <client_type_name>
```

| client_type | client_type_name |
| ----------- | :--------------: |
| Tendermint  |    tendermint    |
| Aggrelite   |    aggrelite     |

**create connection**

```shell
connection create --source <sourece_chain_name> --target <target_chain_name> --sourceclient <source_client
_name> --targetclient <target_client
_name>
```

**create channel**

```shell
channel create --source <sourece_chain_name> --target <target_chain_name> --sourceclient <source_client
_name> --targetclient <target_client
_name> --sourceconn <source_connection
_name> --targetconn <source_connection
_name> --sourceport <sorce_port> --targetport <target_port> --sourceversion <sorce_version> --targetversion <target_version>
```

**aggregator start**

```shell
aggregator start --mode <mode_type> --gtype <num>
```

| mode       | mode_type |
| ---------- | --------- |
| Aggregator | mosaicxc  |
| CosmosIBC  | cosmosibc |

| gtype         | num  |
| ------------- | ---- |
| Non-Grouping  | 0    |
| Random        | 1    |
| ClusterRandom | 2    |


