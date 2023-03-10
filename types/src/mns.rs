//! MNS(Mail name service) association types.

use libipld::{Cid, DagCbor};
use serde::{Deserialize, Serialize};

use crate::{PublicKey, UNSAccount};

/// MNS account information.
#[derive(Default, Debug, Clone, DagCbor, Serialize, Deserialize)]
pub struct MNSAccount {
    /// The nft id of **MNS contract**
    pub uns: UNSAccount,
    /// Account [`type`](MNSTypes) name.
    pub r#type: MNSTypes,
    /// Account bound asymmetrically encryption public key
    pub pub_key: PublicKey,
    /// Account max storage quota in bytes
    pub quota: u64,
    /// Account storage data lease duration in seconds
    pub lease: u64,
    /// multihash for client id.
    pub client_id: Cid,
}

/// MNS account types enum
#[derive(Debug, Serialize, DagCbor, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MNSTypes {
    ///  **MNS** Unicast account.
    Unicast,
    ///  **MNS** Multicast account.
    Multicast,
    ///  **MNS** SP account.
    ServiceProvider,
}

impl Default for MNSTypes {
    fn default() -> Self {
        Self::Unicast
    }
}
