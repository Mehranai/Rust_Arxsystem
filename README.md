# dockerized_eth_code
This is ( ETH MultiThread + ETH Node run (Reth) + Docker Reth + Docker ETH code )

## Create an Network for both Node + Clickhouse

```command
docker network create blockchain-net
```

## Run ETH Node

```command
cd node/Ethereum
docker compose up -d
docker logs -f reth
docker logs -f prysm
```

#### Fixing errors
sudo rm -rf ch-data
------------------
docker compose down -v
------------------
docker compose down
sudo rm -rf ch-data
mkdir ch-data
sudo chown -R 101:101 ch-data
sudo chmod -R 755 ch-data
docker compose up -d
-------------------

### Outputs for Prysm:
    ✔ Connected to execution client
    ✔ Checkpoint sync started
    ✔ Beacon chain syncing

### Outputs for Reth:
    Consensus client connected


## Run App (ETH + Clickhouse)

```command
cd app
APP_MODE=eth docker compose up -d
docker logs -f rust-app
```

## Test if Node and Clickhouse is running acurately

### RPC Ethereum
```command
curl http://localhost:8545
```

### ClickHouse
```command
curl http://localhost:8123
```

-------------------------------------------------

# Binance Smart Chain

In this section we are going to run BSC

## Run Node

```command
cd node/BSC
docker compose up -d
docker logs -f bsc-node
```

### check if node is downloading data

    ✔ Imported new chain segment  blocks=192  txs=12431  elapsed=3.2s
    ✔ Syncing blockchain  imported=123456  elapsed=1h23m


## Run App (BSC + ClickHouse)

```command
cd app
docker compose up -d
docker logs -f rust-app
```

## Test if Node and Clickhouse is running acurately

### RPC BSC
```command
curl http://localhost:8545
```

### ClickHouse
```command
curl http://localhost:8123
```