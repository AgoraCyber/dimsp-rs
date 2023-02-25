use agora_mail_types::{MNSAccount, PublicKey, SPRSAccount};
use async_trait::async_trait;

/// Use this trait to fetch **sp network** information.
#[async_trait]
pub trait SpNetwork {
    type Error: std::error::Error + Sync + Send + 'static;

    /// Search the list of SP service accounts subscribed by `mns`
    ///
    /// Returns [`SPRSAccount`] list.
    async fn sps_subscribed_by(&self, mns: MNSAccount) -> Result<Vec<SPRSAccount>, Self::Error>;

    /// Search mns account by public key. maybe returns [`None`]
    async fn mns_by_pubkey(&self, pubkey: PublicKey) -> Result<Option<MNSAccount>, Self::Error>;

    /// Get mns account by **uns** id.
    async fn mns_by_uns_id(&self, uns_id: u64) -> Result<Option<MNSAccount>, Self::Error>;

    /// Get sprs account by **uns** id.
    async fn sp_by_uns_id(&self, uns_id: u64) -> Result<Option<SPRSAccount>, Self::Error>;
}
