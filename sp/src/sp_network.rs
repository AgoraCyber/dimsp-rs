use async_trait::async_trait;
use dimsp_types::{MNSAccount, PublicKey, SPRSAccount};
use thiserror::Error;

/// Use this trait to fetch **sp network** information.
#[async_trait]
pub trait SpNetwork {
    /// Search the list of SP service accounts subscribed by `mns`
    ///
    /// Returns [`SPRSAccount`] list.
    async fn sps_subscribed_by(&self, mns: MNSAccount) -> anyhow::Result<Option<Vec<SPRSAccount>>>;

    /// Search mns account by public key. maybe returns [`None`]
    async fn mns_by_pubkey(&self, pubkey: PublicKey) -> anyhow::Result<Option<MNSAccount>>;

    /// Get mns account by **uns** id.
    async fn mns_by_uns_id(&self, uns_id: u64) -> anyhow::Result<Option<MNSAccount>>;

    /// Get sprs account by **uns** id.
    async fn sp_by_uns_id(&self, uns_id: u64) -> anyhow::Result<Option<SPRSAccount>>;
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
