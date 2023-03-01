use async_trait::async_trait;
use dimsp_types::{Hash32, MNSAccount};

/// Abstract of blob storage,e.g, **Azure Blob**
/// We assume that the implementation has a lease based GC for storage space.
#[async_trait]
pub trait BlobProvider {
    /// Start writing one blob with lease, Returns blob id
    /// - `blob_hash` blob keccack256 hash.
    async fn start_write_blob(
        &mut self,
        mns: &MNSAccount,
        length: u64,
        fragment_hashes: Vec<Hash32>,
    ) -> anyhow::Result<Blob>;

    /// Write blob fragment
    async fn write_blob_fragment(
        &mut self,
        id: &[u8; 32],
        offset: u64,
        data: Vec<u8>,
    ) -> anyhow::Result<()>;

    /// End writing one blob
    async fn end_write_blob(&mut self, id: &[u8; 32]) -> anyhow::Result<()>;

    /// Manual invoke `GC` to release blob.
    /// Operation may or may not release blob referenced by id,
    /// this depend on how many id reference to this blob
    async fn remove_blob(&mut self, id: &[u8; 32]) -> anyhow::Result<()>;

    async fn start_read_blob(&mut self, id: &[u8; 32]) -> anyhow::Result<()>;

    async fn read_blob_fragment(&mut self, id: &[u8; 32], offset: u64) -> anyhow::Result<Vec<u8>>;
}

/// Blob instance.
#[derive(Debug, Clone)]
pub struct Blob {
    /// Blob unique id
    pub id: [u8; 32],
    /// account.
    pub mns: MNSAccount,
    /// blob total length.
    pub length: u64,
    /// blob hashes
    pub fragment_hashes: Vec<Hash32>,
    /// If next_fragment == fragment count, don't need call write_blob_fragment and end_blob any more.
    pub next_fragment: u64,
}
