CREATE DATABASE IF NOT EXISTS bsc_db;

CREATE TABLE IF NOT EXISTS bsc_db.wallet_info (
    address String,
    balance String,
    nonce UInt64,
    type String,
    person_id String
) ENGINE = ReplacingMergeTree()
ORDER BY address;

CREATE TABLE IF NOT EXISTS bsc_db.transactions (
    hash String,
    block_number UInt64,
    from_addr String,
    to_addr String,
    value String,
    sensivity UInt8
) ENGINE = MergeTree()
ORDER BY block_number;

CREATE TABLE IF NOT EXISTS bsc_db.owner_info (
    address String,
    person_name String,
    person_id String,
    personal_id UInt16
) ENGINE = ReplacingMergeTree()
ORDER BY address;

CREATE TABLE IF NOT EXISTS bsc_db.address_tags (
    address String,
    tag String,
    created_at DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY (address, tag);


/*
New sections
*/
---------------------------------------------------------
-- SYNC STATE
---------------------------------------------------------
CREATE TABLE IF NOT EXISTS bsc_db.sync_state (
    chain String,
    last_synced_block UInt64,
    updated_at DateTime DEFAULT now()
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY chain;