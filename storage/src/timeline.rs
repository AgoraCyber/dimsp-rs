use async_trait::async_trait;
use dimsp_types::MNSAccount;
use libipld::Cid;

use anyhow::Result;

/// Ipld kv database.
#[async_trait]
pub trait Timeline {
    /// Append cid into account's timeline column.
    async fn append(&mut self, mns: MNSAccount, cid: Cid) -> Result<()>;

    /// Get account's first n cids.
    async fn get(&mut self, mns: MNSAccount, first_n: u64) -> Result<Vec<Cid>>;

    /// Move timeline cursor to next `n` cid.
    /// if out of range, the cursor will be set to the end of timeline.
    async fn advance(&mut self, mns: MNSAccount, steps: u64) -> Result<u64>;

    async fn length(&mut self, mns: MNSAccount) -> Result<u64>;
}
