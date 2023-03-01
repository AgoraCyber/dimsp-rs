use sha3::{Digest, Keccak256};

use crate::proto;

impl From<proto::sync::Hash32> for [u8; 32] {
    fn from(value: proto::sync::Hash32) -> Self {
        let mut buff = [0; 32];

        buff[..8].clone_from_slice(&value.h1.to_be_bytes());
        buff[8..16].clone_from_slice(&value.h2.to_be_bytes());
        buff[16..24].clone_from_slice(&value.h3.to_be_bytes());
        buff[24..32].clone_from_slice(&value.h4.to_be_bytes());

        buff
    }
}

impl From<&proto::sync::Hash32> for [u8; 32] {
    fn from(value: &proto::sync::Hash32) -> Self {
        let mut buff = [0; 32];

        buff[..8].clone_from_slice(&value.h1.to_be_bytes());
        buff[8..16].clone_from_slice(&value.h2.to_be_bytes());
        buff[16..24].clone_from_slice(&value.h3.to_be_bytes());
        buff[24..32].clone_from_slice(&value.h4.to_be_bytes());

        buff
    }
}

impl From<[u8; 32]> for proto::sync::Hash32 {
    fn from(value: [u8; 32]) -> Self {
        let mut data = Self::default();

        data.h1 = u64::from_be_bytes(value[..8].try_into().unwrap());
        data.h2 = u64::from_be_bytes(value[8..16].try_into().unwrap());
        data.h3 = u64::from_be_bytes(value[16..24].try_into().unwrap());
        data.h4 = u64::from_be_bytes(value[24..32].try_into().unwrap());

        data
    }
}

/// Keccack256 helper function.
pub fn keccack256(data: &[u8]) -> [u8; 32] {
    Keccak256::new().chain_update(data).finalize().into()
}
