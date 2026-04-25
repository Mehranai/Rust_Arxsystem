CREATE DATABASE IF NOT EXISTS tron_db;

---------------------------------------------------------
-- BLOCKS
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.blocks
(
    block_number UInt64,
    block_hash String,
    parent_hash String,

    tx_count UInt32,
    block_timestamp DateTime,

    producer_address String,

    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY block_number;


---------------------------------------------------------
-- TRANSACTIONS (Native TRX)
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.transactions
(
    tx_hash String,

    block_number UInt64,
    block_timestamp DateTime,

    from_address String,
    to_address String,

    amount UInt64,

    fee UInt64,
    energy_usage UInt64,
    bandwidth_usage UInt64,

    success UInt8,

    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (block_number, tx_hash);


---------------------------------------------------------
-- TRC20 TOKEN TRANSFERS
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.token_transfers
(
    tx_hash String,
    log_index UInt32,

    block_number UInt64,
    block_timestamp DateTime,

    contract_address String,

    from_address String,
    to_address String,

    amount UInt256,

    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (contract_address, block_number);


---------------------------------------------------------
-- CONTRACT CALLS (برای تحلیل رفتار قراردادها)
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.contract_calls
(
    tx_hash String,

    block_number UInt64,
    block_timestamp DateTime,

    caller_address String,
    contract_address String,

    method_id FixedString(8),

    call_value UInt64,
    success UInt8,

    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (contract_address, block_number);


---------------------------------------------------------
-- TOKEN METADATA
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.token_metadata
(
    contract_address String,

    name String,
    symbol String,
    decimals UInt8,

    total_supply UInt256,

    created_at DateTime DEFAULT now(),
    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY contract_address;


---------------------------------------------------------
-- ADDRESS ENTITY TAGGING
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.address_tags
(
    address String,

    tag LowCardinality(String),      -- exchange | mixer | defi | bridge | scam
    source LowCardinality(String),   -- manual | chainalysis | internal
    risk_score UInt8,

    created_at DateTime DEFAULT now()
)
ENGINE = MergeTree()
ORDER BY (address, tag);


---------------------------------------------------------
-- DERIVED BALANCES (برای سریع گرفتن موجودی)
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.address_token_balances
(
    address String,
    contract_address String,

    balance UInt256,

    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (address, contract_address);


---------------------------------------------------------
-- SYNC STATE
---------------------------------------------------------

CREATE TABLE IF NOT EXISTS tron_db.sync_state
(
    last_synced_block UInt64,
    updated_at DateTime DEFAULT now()
)
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY tuple();