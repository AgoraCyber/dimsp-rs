use async_trait::async_trait;

/// Sync stream with persistence support.
#[async_trait]
pub trait Storage {
    type Error: std::error::Error + Sync + Send + 'static;

    type WriterStream;

    /// open receiver stream for `endpoint`
    async fn open_write_stream(&mut self) -> Result<Self::WriterStream, Self::Error>;
}

/// Endpoint sync status.
pub struct SyncStatus {
    pub send_seq_id: u64,
    pub recv_seq_id: u64,
}
