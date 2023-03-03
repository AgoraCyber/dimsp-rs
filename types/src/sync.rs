use libipld::{Cid, DagCbor};
use serde::{Deserialize, Serialize};

#[derive(Debug, DagCbor, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Try push one message referenced by [`cid`](Cid)
    Push(u64, Cid),
    /// Pull cid content multipart
    PullMultipart(u64, Cid),
    /// [`PullMultipart`](SyncMessage::PullMultipart) response
    PullMultipartContent(u64, Mime),
}

#[derive(Debug, DagCbor, Serialize, Deserialize)]
pub struct Mime {
    pub id: Cid,
    pub length: u64,
    pub content: Vec<u8>,
    pub multipart: Vec<Cid>,
}
