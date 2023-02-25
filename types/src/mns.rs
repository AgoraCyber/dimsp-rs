//! MNS(Mail name service) association types.

use serde::{Deserialize, Serialize};

use crate::{PublicKey, UNSAccount};

/// MNS account information.
#[derive(Debug, Serialize, Deserialize)]
pub struct MNSAccount {
    /// The nft id of **MNS contract**
    pub uns: UNSAccount,
    /// Account [`type`](MNSTypes) name.
    pub r#type: MNSTypes,
    /// Account bound asymmetrically encryption public key
    pub pub_key: PublicKey,
}

/// MNS account types enum
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MNSTypes {
    ///  **MNS** Unicast account.
    Unicast,
    ///  **MNS** Multicast account.
    Multicast,
    ///  **MNS** SP account.
    ServiceProvider,
}
