use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockTx {
    pub txid: String,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
}

#[derive(Deserialize)]
pub struct Vin {
    pub prevout: Option<Vout>,
}

#[derive(Deserialize)]
pub struct Vout {
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

#[derive(Deserialize)]
pub struct UTXO { pub value: u64 }