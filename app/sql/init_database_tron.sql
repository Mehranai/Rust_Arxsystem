CREATE DATABASE IF NOT EXISTS tron_db;

---------------------------------------------------------
-- WALLET INFO
---------------------------------------------------------
CREATE TABLE tron_db.wallet_info
(
    address String,
    balance Int64,                     -- TRX in SUN
    wallet_type LowCardinality(String), -- wallet / exchange / smart_contract
    person_id String,
    inserted_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(inserted_at)
ORDER BY address
SETTINGS compression_codec = 'ZSTD(3)';

---------------------------------------------------------
-- TRANSACTIONS (Native TRX)
---------------------------------------------------------
CREATE TABLE tron_db.transactions
(
    hash String,
    block_number UInt64,
    block_time DateTime,
    from_addr String,
    to_addr String,
    value Int64,                     -- TRX in SUN
    sensitivity UInt8,
    inserted_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
PARTITION BY intDiv(block_number, 5_000_000)
ORDER BY (from_addr, block_number, hash)
SETTINGS compression_codec = 'ZSTD(3)';

---------------------------------------------------------
-- OWNER INFO
---------------------------------------------------------
CREATE TABLE tron_db.owner_info
(
    address String,
    person_name String,
    person_id String,
    personal_id UInt16,
    inserted_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(inserted_at)
ORDER BY address;

---------------------------------------------------------
-- ADDRESS TAGS
---------------------------------------------------------
CREATE TABLE tron_db.address_tags
(
    address String,
    tag String,
    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (address, tag);

---------------------------------------------------------
-- TRC20 TRANSFERS (Canonical)
---------------------------------------------------------
CREATE TABLE tron_db.token_transfers
(
    tx_hash String,
    block_number UInt64,
    block_time DateTime,
    log_index UInt32,
    token_address String,
    from_addr String,
    to_addr String,
    amount Int256
)
ENGINE = MergeTree()
PARTITION BY intDiv(block_number, 5_000_000)
ORDER BY (from_addr, token_address, block_number)
SETTINGS compression_codec = 'ZSTD(3)';

---------------------------------------------------------
-- SNAPSHOT BALANCES (Monthly)
---------------------------------------------------------
CREATE TABLE tron_db.address_token_balance_snapshot
(
    snapshot_month Date,
    address String,
    token_address String,
    balance Int256
)
ENGINE = ReplacingMergeTree()
PARTITION BY toYYYYMM(snapshot_month)
ORDER BY (address, token_address);

---------------------------------------------------------
-- TOKEN META DATA
---------------------------------------------------------
CREATE TABLE tron_db.token_metadata
(
    token_address String,
    name String,
    symbol String,
    decimals UInt8,
    total_supply Int256,
    is_verified UInt8,
    created_at DateTime DEFAULT now(),
    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY token_address;

---------------------------------------------------------
-- SYNC STATE
---------------------------------------------------------
CREATE TABLE tron_db.sync_state
(
    chain String,
    last_synced_block UInt64,
    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY chain;

---------------------------------------------------------
-- ADDRESS ENERGY USAGE (AML / Behavior Analysis)
---------------------------------------------------------
CREATE TABLE tron_db.address_energy_usage
(
    address String,
    block_number UInt64,
    energy_usage UInt64,
    energy_fee UInt64,
    net_usage UInt64,
    net_fee UInt64,
    tx_hash String,
    inserted_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(inserted_at)
ORDER BY (address, block_number, tx_hash);

---------------------------------------------------------
-- CONTRACT CALLS (Smart Contract Trace Layer)
---------------------------------------------------------
CREATE TABLE tron_db.contract_calls
(
    tx_hash String,
    block_number UInt64,
    caller_address String,
    contract_address String,
    contract_type LowCardinality(String),        -- TriggerSmartContract / CreateSmartContract
    method_signature String,                     -- optional decoded method
    call_value Int64,                            -- SUN
    energy_used UInt64,
    result LowCardinality(String),               -- SUCCESS / REVERT
    inserted_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(inserted_at)
ORDER BY (block_number, tx_hash, contract_address);

---------------------------------------------------------
-- RAW LOGS (Sliding Window Only)
---------------------------------------------------------
CREATE TABLE tron_db.raw_logs
(
    tx_hash String,
    block_number UInt64,
    block_time DateTime,
    log_index UInt32,
    contract_address String,
    topic0 String,
    topic1 String,
    topic2 String,
    topic3 String,
    data String
)
ENGINE = MergeTree()
PARTITION BY intDiv(block_number, 5_000_000)
ORDER BY (block_number, tx_hash, log_index)
TTL block_time + INTERVAL 14 DAY DELETE;

---------------------------------------------------------
-- CLASSIFIED EVENTS (Swap / Bridge / LP / Mint / Burn)
---------------------------------------------------------
CREATE TABLE tron_db.classified_events
(
    tx_hash String,
    block_number UInt64,
    block_time DateTime,
    event_type LowCardinality(String),          -- swap | mint | burn | bridge | lp_add | lp_remove | unknown
    primary_address String,
    secondary_address String,
    token_address String,
    amount Int256
)
ENGINE = MergeTree()
PARTITION BY intDiv(block_number, 5_000_000)
ORDER BY (primary_address, block_number);