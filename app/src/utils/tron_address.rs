use sha2::{Digest, Sha256};
use hex;

/// Hex (41...) → Base58
pub fn hex_to_base58(hex_addr: &str) -> Option<String> {
    let cleaned = hex_addr.trim_start_matches("0x");

    // handle padded topics (take last 42 chars)
    let normalized = if cleaned.len() > 42 {
        &cleaned[cleaned.len() - 42..]
    } else {
        cleaned
    };

    if normalized.len() != 42 {
        return None;
    }

    let bytes = hex::decode(normalized).ok()?;

    let mut payload = bytes.clone();

    let hash1 = Sha256::digest(&payload);
    let hash2 = Sha256::digest(&hash1);

    let checksum = &hash2[0..4];
    payload.extend_from_slice(checksum);

    Some(bs58::encode(payload).into_string())
}

/// Base58 → Hex (41...)
pub fn base58_to_hex(addr: &str) -> Option<String> {
    let decoded = bs58::decode(addr).into_vec().ok()?;

    if decoded.len() < 4 {
        return None;
    }

    let raw = &decoded[..decoded.len() - 4];

    Some(hex::encode(raw).to_uppercase())
}

/// Normalize address → always Base58
pub fn normalize_tron_address(addr: &str) -> Option<String> {

    if addr.is_empty() {
        return None;
    }

    // already base58
    if addr.starts_with('T') {
        return Some(addr.to_string());
    }

    // try hex conversion (handles padded too)
    hex_to_base58(addr)
}