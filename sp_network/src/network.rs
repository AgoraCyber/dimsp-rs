use async_trait::async_trait;
use dimsp_types::{PublicKey, SPRSAccount};
use thiserror::Error;

/// Use this trait to fetch **sp network** information.
#[async_trait]
pub trait SpNetwork {
    /// Get subscription list of `mns_id`
    ///
    /// Returns [`SPRSAccount`] list.
    async fn subscription(&self, mns_id: u64) -> anyhow::Result<Option<Vec<SPRSAccount>>>;
}

#[derive(Debug, Error)]
pub enum SpNetworkError {
    #[error("MNS_BY_PUB_KEY: Account({0}) not found")]
    MNSByPubKey(PublicKey),
    #[error("MNSById: Account({0}) not found")]
    MNSById(u64),
    #[error("SPRSById: Account({0}) not found")]
    SPRSById(u64),
}
