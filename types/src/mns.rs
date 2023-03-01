//! MNS(Mail name service) association types.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{PublicKey, UNSAccount};

/// MNS account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MNSAccount {
    /// The nft id of **MNS contract**
    pub uns: UNSAccount,
    /// Account [`type`](MNSTypes) name.
    pub r#type: MNSTypes,
    /// Account bound asymmetrically encryption public key
    pub pub_key: PublicKey,
    /// Account max storage quota in bytes
    pub quota: u64,
    /// Account storage data lease duration
    pub lease: Duration,
    /// Using client local public key as client_id
    pub client_id: PublicKey,
}

/// MNS account types enum
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MNSTypes {
    ///  **MNS** Unicast account.
    Unicast,
    ///  **MNS** Multicast account.
    Multicast,
    ///  **MNS** SP account.
    ServiceProvider,
}
