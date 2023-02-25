use std::fmt::Display;

use hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};

use crate::proto;

#[derive(Debug, thiserror::Error)]
pub enum SyncEndpointError {
    #[error("Sync endpoint deserialize length(32) mismatch")]
    DeserializeLength(String),
}

pub struct SyncEndpoint([u8; 32]);

impl From<proto::sync::Hash32> for SyncEndpoint {
    fn from(value: proto::sync::Hash32) -> Self {
        let mut buff = [0; 32];

        buff[..8].clone_from_slice(&value.h1.to_be_bytes());
        buff[8..16].clone_from_slice(&value.h2.to_be_bytes());
        buff[16..24].clone_from_slice(&value.h3.to_be_bytes());
        buff[24..32].clone_from_slice(&value.h4.to_be_bytes());

        Self(buff)
    }
}

impl From<SyncEndpoint> for proto::sync::Hash32 {
    fn from(value: SyncEndpoint) -> Self {
        let mut data = Self::default();

        data.h1 = u64::from_be_bytes(value.0[..8].try_into().unwrap());
        data.h2 = u64::from_be_bytes(value.0[8..16].try_into().unwrap());
        data.h3 = u64::from_be_bytes(value.0[16..24].try_into().unwrap());
        data.h4 = u64::from_be_bytes(value.0[24..32].try_into().unwrap());

        data
    }
}

impl Display for SyncEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.0.encode_hex::<String>())
    }
}

impl Serialize for SyncEndpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SyncEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;

        let buff = Vec::<u8>::from_hex(&text.trim_start_matches("0x"))
            .map_err(serde::de::Error::custom)?;

        if buff.len() != 32 {
            return Err(SyncEndpointError::DeserializeLength(text))
                .map_err(serde::de::Error::custom);
        }

        let buff: [u8; 32] = buff.try_into().unwrap();

        Ok(Self(buff))
    }
}
