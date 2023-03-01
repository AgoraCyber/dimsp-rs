use async_trait::async_trait;
use dimsp_types::{Inbox, MNSAccount};

use crate::blob::Blob;

/// Timeline fifo metdata queue.
#[async_trait]
pub trait TimelineProvider {
    /// Append one stream metadata. returns timeline length.
    async fn append(&mut self, blob: Blob) -> anyhow::Result<u64>;

    /// Get the first blob metadata in timeline of [`account`](MNSAccount)
    async fn get(&mut self, account: &MNSAccount) -> anyhow::Result<Option<Blob>>;

    /// Pop the first blob metadata in timeline of [`account`](MNSAccount)
    async fn pop(&mut self, account: &MNSAccount) -> anyhow::Result<()>;

    /// Get [`account`](MNSAccount) statuts infomation
    async fn status(&mut self, account: &MNSAccount) -> anyhow::Result<Inbox>;
}
