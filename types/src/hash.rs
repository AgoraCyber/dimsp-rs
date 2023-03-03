use sha3::{Digest, Keccak256};

/// Keccack256 helper function.
pub fn keccack256(data: &[u8]) -> [u8; 32] {
    Keccak256::new().chain_update(data).finalize().into()
}
