//! UNS(User name service) association types.

use serde::{Deserialize, Serialize};

/// MNS account information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UNSAccount {
    /// The nft id of **UNS contract**
    pub id: u64,
    /// display user name for **UNS account** .
    pub user_name: String,
}
