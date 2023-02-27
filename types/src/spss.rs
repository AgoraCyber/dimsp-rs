//! SPRS(SP register service) association types.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::{MNSAccount, PublicKey};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SPRSAccount {
    pub mns: MNSAccount,
    /// SPRS endpoints.
    pub endpoints: Vec<SPRSEndpoint>,
}

/// SPRS node endpoint definition
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SPRSEndpoint {
    /// Endpoint listening addr.
    pub addr: SocketAddr,
    /// Endpoint handshake public key.
    pub pub_key: PublicKey,
}
