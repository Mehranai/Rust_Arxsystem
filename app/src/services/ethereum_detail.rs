use std::collections::HashMap;
use ethers::prelude::*;
use ethers::utils::keccak256;
use ethers::types::{Address, U256};

#[derive(Debug, Clone)]
pub enum TxCategory {
    Failed, // 1
    Approve, // 2
    NFTTransfer, // 3
    Swap, // 4
    LiquidityPool, // 5
    Bridge, // 6
    Stake, // 7
    ERC20Transfer, // 8
    EthTransfer, // 9
    Other, // 10
}

pub fn classify_tx(tx: &Transaction, receipt: &TransactionReceipt) -> TxCategory {

    if is_failed(receipt) {
        return TxCategory::Failed;
    }

    // --- Layer 1 (Hard) ---
    if is_approve(tx, receipt) {
        return TxCategory::Approve;
    }

    if is_nft_transfer(receipt) {
        return TxCategory::NFTTransfer;
    }

    if is_liquidity_pool(tx, receipt) {
        return TxCategory::LiquidityPool;
    }

    if is_swap(tx, receipt) {
        return TxCategory::Swap;
    }

    // --- Layer 2 (Intent-based) ---
    if is_stake(tx, receipt) {
        return TxCategory::Stake;
    }

    if is_bridge(tx, receipt) {
        return TxCategory::Bridge;
    }

    // --- Layer 3 ---
    if is_erc20_transfer(tx, receipt) {
        return TxCategory::ERC20Transfer;
    }

    if is_eth_transfer(tx, receipt) {
        return TxCategory::EthTransfer;
    }

    TxCategory::Other
}

// ------------ Signeture -----------------
fn erc20_transfer_sig() -> H256 {
    H256::from(keccak256("Transfer(address,address,uint256)"))
}

fn weth_deposit_sig() -> H256 {
    H256::from(keccak256("Deposit(address,uint256)"))
}

fn weth_withdraw_sig() -> H256 {
    H256::from(keccak256("Withdrawal(address,uint256)"))
}

fn approve_sig() -> H256 {
    H256::from(keccak256("Approval(address,address,uint256)"))
}

// --------------- Failed -----------------

pub fn is_failed(receipt: &TransactionReceipt) -> bool {
    receipt.status != Some(U64::from(1))
}

// --------------- NFT ---------------------
pub fn is_nft_transfer(receipt: &TransactionReceipt) -> bool {
    let erc721_sig = H256::from(keccak256(
        "Transfer(address,address,uint256)"
    ));
    let erc1155_single = H256::from(keccak256(
        "TransferSingle(address,address,address,uint256,uint256)"
    ));
    let erc1155_batch = H256::from(keccak256(
        "TransferBatch(address,address,address,uint256[],uint256[])"
    ));

    receipt.logs.iter().any(|log| {
        (log.topics.len() == 4 && log.topics[0] == erc721_sig)
            || log.topics[0] == erc1155_single
            || log.topics[0] == erc1155_batch
    })
}

// ---------------- LP ---------------------

#[derive(Debug, Clone)]
pub enum LiquidityAction {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub struct LiquidityPoolDetails {
    pub action: LiquidityAction,
    pub pool: Address,        // LP token contract
    pub token0: Address,
    pub token1: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub lp_amount: U256,
}

#[derive(Default)]
struct TokenFlow {
    sent: HashMap<Address, U256>,
    received: HashMap<Address, U256>,
}

fn collect_token_flows(
    receipt: &TransactionReceipt,
    user: Address,
) -> TokenFlow {
    let mut flow = TokenFlow::default();

    for log in &receipt.logs {
        if log.topics.len() != 3 || log.topics[0] != erc20_transfer_sig() {
            continue;
        }

        let from = Address::from_slice(&log.topics[1][12..]);
        let to   = Address::from_slice(&log.topics[2][12..]);
        let amount = U256::from_big_endian(&log.data.0);
        let token  = log.address;

        if from == user {
            *flow.sent.entry(token).or_insert(U256::zero()) += amount;
        }

        if to == user {
            *flow.received.entry(token).or_insert(U256::zero()) += amount;
        }
    }

    flow
}

fn detect_lp_add(
    flow: &TokenFlow,
) -> Option<LiquidityPoolDetails> {

    // LP mint → received
    if flow.received.len() != 1 || flow.sent.len() != 2 {
        return None;
    }

    let (&lp_token, &lp_amount) =
        flow.received.iter().next().unwrap();

    let mut sent_iter = flow.sent.iter();
    let (&token0, &amount0) = sent_iter.next().unwrap();
    let (&token1, &amount1) = sent_iter.next().unwrap();

    Some(LiquidityPoolDetails {
        action: LiquidityAction::Add,
        pool: lp_token,
        token0,
        token1,
        amount0,
        amount1,
        lp_amount,
    })
}

fn detect_lp_remove(
    flow: &TokenFlow,
) -> Option<LiquidityPoolDetails> {

    // LP burn → sent
    if flow.sent.len() != 1 || flow.received.len() != 2 {
        return None;
    }

    let (&lp_token, &lp_amount) =
        flow.sent.iter().next().unwrap();

    let mut recv_iter = flow.received.iter();
    let (&token0, &amount0) = recv_iter.next().unwrap();
    let (&token1, &amount1) = recv_iter.next().unwrap();

    Some(LiquidityPoolDetails {
        action: LiquidityAction::Remove,
        pool: lp_token,
        token0,
        token1,
        amount0,
        amount1,
        lp_amount,
    })
}

pub fn extract_liquidity_pool(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<LiquidityPoolDetails> {

    let user = tx.from;

    // swap / bridge نباید باشد
    if is_swap(tx, receipt) || is_bridge(tx, receipt) {
        return None;
    }

    let flow = collect_token_flows(receipt, user);

    detect_lp_add(&flow).or_else(|| detect_lp_remove(&flow))
}

pub fn is_liquidity_pool(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_liquidity_pool(tx, receipt).is_some()
}
// --------------- Bridge ------------------

#[derive(Debug, Clone)]
pub enum BridgeAsset {
    Eth,
    Erc20(Address),
}

#[derive(Debug, Clone)]
pub struct BridgeDetails {
    pub user: Address,
    pub asset: BridgeAsset,
    pub amount: U256,
    pub bridge_contract: Address,
    pub event_sig: H256,
}

const BRIDGE_EVENT_SIGNATURES: [&str; 5] = [
    "Deposit(address,uint256)",
    "Deposit(address,address,uint256)",
    "Send(address,uint256,uint64)",
    "Locked(address,uint256)",
    "MessageSent(bytes)",
];


use ethers::types::H256;

fn is_bridge_event(sig: H256) -> bool {
    BRIDGE_EVENT_SIGNATURES.iter().any(|s| {
        H256::from(keccak256(s.as_bytes())) == sig
    })
}

fn find_bridge_event<'a>(
    receipt: &'a TransactionReceipt,
) -> Option<&'a ethers::types::Log> {
    receipt.logs.iter().find(|log| {
        is_bridge_event(log.topics[0])
    })
}

pub fn extract_bridge_details(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<BridgeDetails> {

    let user = tx.from;

    let bridge_log = find_bridge_event(receipt)?;
    let bridge_contract = bridge_log.address;
    let event_sig = bridge_log.topics[0];

    let (asset, amount) =
        extract_bridge_asset(tx, receipt, user, bridge_contract)?;

    Some(BridgeDetails {
        user,
        asset,
        amount,
        bridge_contract,
        event_sig,
    })
}

fn extract_bridge_asset(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    user: Address,
    bridge_contract: Address,
) -> Option<(BridgeAsset, U256)> {

    // --- ERC20 ---
    for log in &receipt.logs {
        if log.topics.len() == 3 && log.topics[0] == erc20_transfer_sig() {
            let from = Address::from_slice(&log.topics[1][12..]);
            let to   = Address::from_slice(&log.topics[2][12..]);

            if from == user && to == bridge_contract {
                let amount = U256::from_big_endian(&log.data.0);
                return Some((BridgeAsset::Erc20(log.address), amount));
            }
        }
    }

    // --- ETH ---
    if tx.value > U256::zero() && tx.to == Some(bridge_contract) {
        return Some((BridgeAsset::Eth, tx.value));
    }

    None
}
pub fn is_bridge(tx: &Transaction, receipt: &TransactionReceipt) -> bool {
    extract_bridge_details(tx, receipt).is_some()
}

// ----------------------------------------
// ----------------- Swap -----------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Asset {
    Eth,
    Erc20(Address),
}

#[derive(Debug, Clone)]
pub struct AssetAmount {
    pub asset: Asset,
    pub amount: U256,
}

#[derive(Debug, Clone)]
pub struct SwapDetails {
    pub user: Address,
    pub sent: Vec<AssetAmount>,
    pub received: Vec<AssetAmount>,
}

#[derive(Default)]
struct BalanceDelta {
    sent: HashMap<Asset, U256>,
    received: HashMap<Asset, U256>,
}

fn collect_balance_deltas(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    user: Address,
) -> BalanceDelta {
    let mut delta = BalanceDelta::default();

    // -------- ETH --------
    if tx.value > U256::zero() {
        *delta.sent.entry(Asset::Eth).or_insert(U256::zero()) += tx.value;
    }

    for log in &receipt.logs {
        // -------- ERC20 Transfer --------
        if log.topics.len() == 3 && log.topics[0] == erc20_transfer_sig() {
            let from = Address::from_slice(&log.topics[1][12..]);
            let to   = Address::from_slice(&log.topics[2][12..]);
            let amount = U256::from_big_endian(&log.data.0);
            let token = log.address;

            if from == user {
                *delta.sent
                    .entry(Asset::Erc20(token))
                    .or_insert(U256::zero()) += amount;
            }

            if to == user {
                *delta.received
                    .entry(Asset::Erc20(token))
                    .or_insert(U256::zero()) += amount;
            }
        }

        // -------- WETH unwrap --------
        if log.topics[0] == weth_withdraw_sig() {
            let amount = U256::from_big_endian(&log.data.0);
            *delta.received
                .entry(Asset::Eth)
                .or_insert(U256::zero()) += amount;
        }

        // -------- WETH wrap --------
        if log.topics[0] == weth_deposit_sig() {
            let amount = U256::from_big_endian(&log.data.0);
            *delta.sent
                .entry(Asset::Eth)
                .or_insert(U256::zero()) += amount;
        }
    }

    delta
}

fn normalize_deltas(
    delta: BalanceDelta,
) -> (HashMap<Asset, U256>, HashMap<Asset, U256>) {

    let mut sent_out = HashMap::new();
    let mut recv_out = HashMap::new();

    // همه assetهایی که درگیر بودن
    let mut assets = std::collections::HashSet::new();

    for asset in delta.sent.keys() {
        assets.insert(asset.clone());
    }
    for asset in delta.received.keys() {
        assets.insert(asset.clone());
    }

    for asset in assets {
        let sent_amt = delta.sent.get(&asset).cloned().unwrap_or(U256::zero());
        let recv_amt = delta.received.get(&asset).cloned().unwrap_or(U256::zero());

        if sent_amt > recv_amt {
            sent_out.insert(asset, sent_amt - recv_amt);
        } else if recv_amt > sent_amt {
            recv_out.insert(asset, recv_amt - sent_amt);
        }
    }

    (sent_out, recv_out)
}

fn is_real_swap(
    sent: &HashMap<Asset, U256>,
    received: &HashMap<Asset, U256>,
) -> bool {
    if sent.is_empty() || received.is_empty() {
        return false;
    }

    for s in sent.keys() {
        for r in received.keys() {
            if s != r {
                return true;
            }
        }
    }

    false
}
pub fn extract_swap_details(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<SwapDetails> {

    let user = tx.from;

    let delta = collect_balance_deltas(tx, receipt, user);
    let (sent_map, recv_map) = normalize_deltas(delta);

    if !is_real_swap(&sent_map, &recv_map) {
        return None;
    }

    let sent = sent_map
        .into_iter()
        .map(|(asset, amount)| AssetAmount { asset, amount })
        .collect();

    let received = recv_map
        .into_iter()
        .map(|(asset, amount)| AssetAmount { asset, amount })
        .collect();

    Some(SwapDetails {
        user,
        sent,
        received,
    })
}

pub fn is_swap(tx: &Transaction, receipt: &TransactionReceipt) -> bool {
    extract_swap_details(tx, receipt).is_some()
}

// ---------------------------------------
// --------- ERC20 Token Transfer --------

#[derive(Debug, Clone)]
pub struct ERC20TransferDetails {
    pub token: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
}

fn extract_erc20_transfers(
    receipt: &TransactionReceipt,
) -> Vec<ERC20TransferDetails> {

    let mut out = Vec::new();
    let sig = erc20_transfer_sig();

    for log in &receipt.logs {
        if log.topics.len() != 3 || log.topics[0] != sig {
            continue;
        }

        let from = Address::from_slice(&log.topics[1][12..]);
        let to   = Address::from_slice(&log.topics[2][12..]);
        let amount = U256::from_big_endian(&log.data.0);

        // mint / burn حذف
        if from == Address::zero() || to == Address::zero() {
            continue;
        }

        out.push(ERC20TransferDetails {
            token: log.address,
            from,
            to,
            amount,
        });
    }

    out
}

pub fn extract_simple_erc20_transfer(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<ERC20TransferDetails> {

    let user = tx.from;

    // ❌ نباید category دیگه‌ای باشه
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_stake(tx, receipt)
        || is_nft_transfer(receipt)
    {
        return None;
    }

    let transfers = extract_erc20_transfers(receipt);

    // باید دقیقاً یکی باشه
    if transfers.len() != 1 {
        return None;
    }

    let t = transfers.into_iter().next().unwrap();

    // user باید درگیر باشه
    if t.from != user && t.to != user {
        return None;
    }

    // ETH نباید جابجا شده باشه
    if tx.value > U256::zero() {
        return None;
    }

    Some(t)
}

pub fn is_erc20_transfer(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_simple_erc20_transfer(tx, receipt).is_some()
}

// ---------------------------------
// --------- ETH Transfer ----------

#[derive(Debug, Clone)]
pub struct EthTransferDetails {
    pub from: Address,
    pub to: Address,
    pub amount: U256,
}

fn has_weth_wrap_or_unwrap(
    receipt: &TransactionReceipt,
) -> bool {
    let deposit = weth_deposit_sig();
    let withdraw = weth_withdraw_sig();

    receipt.logs.iter().any(|log| {
        log.topics.get(0) == Some(&deposit)
            || log.topics.get(0) == Some(&withdraw)
    })
}

pub fn extract_simple_eth_transfer(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<EthTransferDetails> {

    // باید ETH جابجا شده باشه
    if tx.value == U256::zero() {
        return None;
    }

    let to = tx.to?; // contract creation رد

    //  wrap / unwrap
    if has_weth_wrap_or_unwrap(receipt) {
        return None;
    }

    //  سایر categoryها
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_stake(tx, receipt)
        || is_erc20_transfer(tx, receipt)
        || is_nft_transfer(receipt)
        || is_approve(tx, receipt)
    {
        return None;
    }

    Some(EthTransferDetails {
        from: tx.from,
        to,
        amount: tx.value,
    })
}
pub fn is_eth_transfer(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_simple_eth_transfer(tx, receipt).is_some()
}

// ------------------------------------
// ------------- Approve --------------

#[derive(Debug, Clone)]
pub struct ApproveDetails {
    pub token: Address,
    pub owner: Address,
    pub spender: Address,
    pub amount: U256,
}

fn extract_approve_events(
    receipt: &TransactionReceipt,
    user: Address,
) -> Vec<ApproveDetails> {

    let mut out = Vec::new();
    let sig = approve_sig();

    for log in &receipt.logs {
        if log.topics.len() != 3 || log.topics[0] != sig {
            continue;
        }

        let owner   = Address::from_slice(&log.topics[1][12..]);
        let spender = Address::from_slice(&log.topics[2][12..]);
        let amount  = U256::from_big_endian(&log.data.0);

        if owner != user {
            continue;
        }

        out.push(ApproveDetails {
            token: log.address,
            owner,
            spender,
            amount,
        });
    }

    out
}

fn allowance_used_in_same_tx(
    receipt: &TransactionReceipt,
    user: Address,
    token: Address,
) -> bool {

    let transfer_sig =
        H256::from(keccak256("Transfer(address,address,uint256)"));

    receipt.logs.iter().any(|log| {
        log.topics.len() == 3
            && log.topics[0] == transfer_sig
            && log.address == token
            && Address::from_slice(&log.topics[1][12..]) == user
    })
}

pub fn extract_standalone_approve(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<ApproveDetails> {

    let user = tx.from;

    // ❌ سایر categoryها
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_stake(tx, receipt)
        || is_erc20_transfer(tx, receipt)
        || is_nft_transfer(receipt)
    {
        return None;
    }

    let approves = extract_approve_events(receipt, user);

    // باید دقیقاً یکی باشه
    if approves.len() != 1 {
        return None;
    }

    let approve = approves.into_iter().next().unwrap();

    // allowance نباید در همان tx مصرف شده باشد
    if allowance_used_in_same_tx(receipt, user, approve.token) {
        return None;
    }

    Some(approve)
}

pub fn is_approve(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_standalone_approve(tx, receipt).is_some()
}

// -----------------------------------
// ----------- deposit stake ---------

#[derive(Debug, Clone)]
pub struct StakeDetails {
    pub user: Address,
    pub staking_contract: Address,

    pub sent_asset: Asset,
    pub sent_amount: U256,

    pub received_asset: Option<Asset>,
    pub received_amount: Option<U256>,
}

fn stake_method_selectors() -> Vec<[u8; 4]> {
    vec![
        keccak256("stake(uint256)")[0..4].try_into().unwrap(),
        keccak256("deposit(uint256)")[0..4].try_into().unwrap(),
        keccak256("deposit(address,uint256,address,uint16)")[0..4].try_into().unwrap(),
        keccak256("submit(address)")[0..4].try_into().unwrap(), // Lido
        keccak256("lock(uint256)")[0..4].try_into().unwrap(),
    ]
}

fn has_stake_intent(tx: &Transaction) -> bool {
    let input = &tx.input.0;

    if input.len() < 4 {
        return false;
    }

    let selector: [u8; 4] = input[0..4].try_into().unwrap();

    stake_method_selectors()
        .iter()
        .any(|s| *s == selector)
}
fn extract_stake_flows(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    user: Address,
) -> Option<(
    Asset,              // sent_asset
    U256,               // sent_amount
    Option<(Asset, U256)>, // received (optional)
)> {
    let staking_contract = tx.to?;

    // ---------------- ETH stake ----------------
    
    if tx.value > U256::zero() {
        return Some((
            Asset::Eth,
            tx.value,
            None,
        ));
    }

    // ---------------- ERC20 stake ----------------
    let mut sent: Option<(Asset, U256)> = None;
    let mut received: Option<(Asset, U256)> = None;

    for log in &receipt.logs {
        // فقط ERC20 Transfer
        if log.topics.len() != 3 {
            continue;
        }
        if log.topics[0] != erc20_transfer_sig() {
            continue;
        }

        let from = Address::from_slice(&log.topics[1][12..]);
        let to   = Address::from_slice(&log.topics[2][12..]);
        let amount = U256::from_big_endian(&log.data.0);
        let token = log.address;

        // user -> staking contract  (دارایی staked)
        if from == user && to == staking_contract {
            sent = Some((Asset::Erc20(token), amount));
            continue;
        }

        // mint receipt token (stETH, aToken, etc)
        if from == Address::zero() && to == user {
            received = Some((Asset::Erc20(token), amount));
        }
    }

    let (sent_asset, sent_amount) = sent?;

    Some((
        sent_asset,
        sent_amount,
        received,
    ))
}

pub fn extract_stake_details(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<StakeDetails> {

    let user = tx.from;
    let staking_contract = tx.to?;

    // hard exclusions 
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_nft_transfer(receipt)
    {
        return None;
    }

    // intent must exist
    if !has_stake_intent(tx) {
        return None;
    }

    let (sent_asset, sent_amount, received) =
        extract_stake_flows(tx, receipt, user)?;

    Some(StakeDetails {
        user,
        staking_contract,
        sent_asset,
        sent_amount,
        received_asset: received.as_ref().map(|r| r.0.clone()),
        received_amount: received.as_ref().map(|r| r.1),
    })
}
pub fn is_stake(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_stake_details(tx, receipt).is_some()
}

// ---------------------------------
// ------------ withdraw -----------

#[derive(Debug, Clone)]
pub struct WithdrawDetails {
    pub user: Address,
    pub staking_contract: Address,

    pub received_asset: Asset,
    pub received_amount: U256,

    pub burned_asset: Option<Asset>,
    pub burned_amount: Option<U256>,
}
fn withdraw_method_selectors() -> Vec<[u8; 4]> {
    vec![
        keccak256("withdraw(uint256)")[0..4].try_into().unwrap(),
        keccak256("withdraw(address,uint256)")[0..4].try_into().unwrap(),
        keccak256("unstake(uint256)")[0..4].try_into().unwrap(),
        keccak256("redeem(uint256)")[0..4].try_into().unwrap(),
        keccak256("exit()")[0..4].try_into().unwrap(),
    ]
}
fn has_withdraw_intent(tx: &Transaction) -> bool {
    let input = &tx.input.0;

    if input.len() < 4 {
        return false;
    }

    let selector: [u8; 4] = input[0..4].try_into().unwrap();

    withdraw_method_selectors()
        .iter()
        .any(|s| *s == selector)
}
fn extract_withdraw_flows(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    user: Address,
) -> Option<(
    Asset,              // received_asset
    U256,               // received_amount
    Option<(Asset, U256)>, // burned (receipt token)
)> {
    let staking_contract = tx.to?;

    let mut received: Option<(Asset, U256)> = None;
    let mut burned: Option<(Asset, U256)> = None;

    // ---------------- ETH receive ----------------
   
    for log in &receipt.logs {

        if log.topics.get(0) == Some(&weth_withdraw_sig()) {
            let amount = U256::from_big_endian(&log.data.0);
            received = Some((Asset::Eth, amount));
        }
    }

    // ---------------- ERC20 flows ----------------
    for log in &receipt.logs {
        if log.topics.len() != 3 {
            continue;
        }
        if log.topics[0] != erc20_transfer_sig() {
            continue;
        }

        let from = Address::from_slice(&log.topics[1][12..]);
        let to   = Address::from_slice(&log.topics[2][12..]);
        let amount = U256::from_big_endian(&log.data.0);
        let token = log.address;

        // staking contract -> user (asset received)
        if from == staking_contract && to == user {
            received = Some((Asset::Erc20(token), amount));
            continue;
        }

        // receipt token burned (user -> zero)
        if from == user && to == Address::zero() {
            burned = Some((Asset::Erc20(token), amount));
        }
    }

    let (received_asset, received_amount) = received?;

    Some((
        received_asset,
        received_amount,
        burned,
    ))
}
pub fn extract_withdraw_details(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<WithdrawDetails> {

    let user = tx.from;
    let staking_contract = tx.to?;

    // ❌ hard exclusions
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_nft_transfer(receipt)
    {
        return None;
    }

    // intent required
    if !has_withdraw_intent(tx) {
        return None;
    }

    let (received_asset, received_amount, burned) =
        extract_withdraw_flows(tx, receipt, user)?;

    Some(WithdrawDetails {
        user,
        staking_contract,
        received_asset,
        received_amount,
        burned_asset: burned.as_ref().map(|b| b.0.clone()),
        burned_amount: burned.as_ref().map(|b| b.1),
    })
}
pub fn is_withdraw(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_withdraw_details(tx, receipt).is_some()
}

// ---------------------------------
// ------------ deposit ------------

#[derive(Debug, Clone)]
pub struct DepositDetails {
    pub user: Address,
    pub contract: Address,

    pub deposited_asset: Asset,
    pub deposited_amount: U256,

    pub received_asset: Option<Asset>,   // aToken, cToken, vault share
    pub received_amount: Option<U256>,
}
fn deposit_method_selectors() -> Vec<[u8; 4]> {
    vec![
        keccak256("deposit(uint256)")[0..4].try_into().unwrap(),
        keccak256("deposit(address,uint256)")[0..4].try_into().unwrap(),
        keccak256("deposit(address,uint256,address)")[0..4].try_into().unwrap(),
        keccak256("supply(address,uint256,address,uint16)")[0..4].try_into().unwrap(), // Aave
        keccak256("mint(uint256)")[0..4].try_into().unwrap(), // Compound
    ]
}
fn has_deposit_intent(tx: &Transaction) -> bool {
    let input = &tx.input.0;

    if input.len() < 4 {
        return false;
    }

    let selector: [u8; 4] = input[0..4].try_into().unwrap();

    deposit_method_selectors()
        .iter()
        .any(|s| *s == selector)
}
fn extract_deposit_flows(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    user: Address,
) -> Option<(
    Asset,
    U256,
    Option<(Asset, U256)>,
)> {
    let contract = tx.to?;

    // ---------------- ETH deposit ----------------
    if tx.value > U256::zero() {
        return Some((
            Asset::Eth,
            tx.value,
            None,
        ));
    }

    let mut deposited: Option<(Asset, U256)> = None;
    let mut received: Option<(Asset, U256)> = None;

    for log in &receipt.logs {
        if log.topics.len() != 3 {
            continue;
        }
        if log.topics[0] != erc20_transfer_sig() {
            continue;
        }

        let from = Address::from_slice(&log.topics[1][12..]);
        let to   = Address::from_slice(&log.topics[2][12..]);
        let amount = U256::from_big_endian(&log.data.0);
        let token = log.address;

        // user -> deposit contract
        if from == user && to == contract {
            deposited = Some((Asset::Erc20(token), amount));
            continue;
        }

        // mint receipt token (aToken, cToken, vault share)
        if from == Address::zero() && to == user {
            received = Some((Asset::Erc20(token), amount));
        }
    }

    let (asset, amount) = deposited?;

    Some((asset, amount, received))
}
pub fn extract_deposit_details(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> Option<DepositDetails> {

    let user = tx.from;
    let contract = tx.to?;

    // ❌ exclusions (خیلی مهم)
    if is_swap(tx, receipt)
        || is_bridge(tx, receipt)
        || is_liquidity_pool(tx, receipt)
        || is_stake(tx, receipt)
        || is_withdraw(tx, receipt)
        || is_nft_transfer(receipt)
    {
        return None;
    }

    // intent required
    if !has_deposit_intent(tx) {
        return None;
    }

    let (asset, amount, received) =
        extract_deposit_flows(tx, receipt, user)?;

    Some(DepositDetails {
        user,
        contract,
        deposited_asset: asset,
        deposited_amount: amount,
        received_asset: received.as_ref().map(|r| r.0.clone()),
        received_amount: received.as_ref().map(|r| r.1),
    })
}
pub fn is_deposit(
    tx: &Transaction,
    receipt: &TransactionReceipt,
) -> bool {
    extract_deposit_details(tx, receipt).is_some()
}

// ----------------------------------