use std::fmt::Display;

use hex::{FromHex, ToHex};

use libipld::{
    cbor::DagCborCodec,
    prelude::{Decode, Encode},
    DagCbor,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum PublicKeyError {
    #[error("MNS public key deserialize length({0}) error, {1}")]
    DeserializeLength(usize, String),
}

#[derive(Debug, Serialize, DagCbor, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum PublicKey {
    /// rsa1024 public key
    #[serde(rename = "rsa1024")]
    RSA1024(PublicKeyBuff<128>),
    /// rsa2048 public key
    #[serde(rename = "rsa2048")]
    RSA2048(PublicKeyBuff<256>),
    /// rsa4096 public key
    #[serde(rename = "rsa4096")]
    RSA4096(PublicKeyBuff<512>),
    /// Ed25519 public key
    #[serde(rename = "ed25519")]
    Ed25519(PublicKeyBuff<32>),
    /// ECSDA public key
    #[serde(rename = "ecdsa")]
    ECDSA(PublicKeyBuff<33>),
}

impl Default for PublicKey {
    fn default() -> Self {
        Self::ECDSA(Default::default())
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RSA1024(value) => {
                write!(f, "RSA1024: {}", value)
            }
            Self::RSA2048(value) => {
                write!(f, "RSA2048: {}", value)
            }
            Self::RSA4096(value) => {
                write!(f, "RSA4096: {}", value)
            }
            Self::Ed25519(value) => {
                write!(f, "Ed25519: {}", value)
            }
            Self::ECDSA(value) => {
                write!(f, "ECDSA: {}", value)
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PublicKeyBuff<const LEN: usize>(pub [u8; LEN]);

impl<const LEN: usize> Encode<DagCborCodec> for PublicKeyBuff<LEN> {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        Ok(self.0.to_vec().encode(c, w)?)
    }
}

impl<const LEN: usize> Decode<DagCborCodec> for PublicKeyBuff<LEN> {
    fn decode<R: std::io::Read + std::io::Seek>(
        c: DagCborCodec,
        r: &mut R,
    ) -> anyhow::Result<Self> {
        let buff = Vec::<u8>::decode(c, r)?;

        if buff.len() != LEN {
            return Err(anyhow::format_err!(
                "DagCbor: read public key({}) length error, {}",
                LEN,
                buff.len(),
            ));
        }

        return Ok(PublicKeyBuff(buff.try_into().unwrap()));
    }
}

impl<const LEN: usize> Default for PublicKeyBuff<LEN> {
    fn default() -> Self {
        Self([0u8; LEN])
    }
}

impl<const LEN: usize> Display for PublicKeyBuff<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.0.encode_hex::<String>())
    }
}

impl<const LEN: usize> Serialize for PublicKeyBuff<LEN> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de, const LEN: usize> Deserialize<'de> for PublicKeyBuff<LEN> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;

        let buff = Vec::<u8>::from_hex(&text.trim_start_matches("0x"))
            .map_err(serde::de::Error::custom)?;

        if buff.len() != LEN {
            return Err(PublicKeyError::DeserializeLength(LEN, text))
                .map_err(serde::de::Error::custom);
        }

        let buff: [u8; LEN] = buff.try_into().unwrap();

        Ok(Self(buff))
    }
}
