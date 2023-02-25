use agora_mail_types::{MNSAccount, OpenStream, SyncEndpoint};
use async_trait::async_trait;

/// Sync stream with persistence support.
#[async_trait]
pub trait PersistenceSyncStream {
    type Error: std::error::Error + Sync + Send + 'static;

    type WriterStream;

    /// Handshake and return sync status with [`endpoint`](SyncEndpoint)
    async fn handshake(
        &mut self,
        endpoint: SyncEndpoint,
        mns: MNSAccount,
    ) -> Result<SyncStatus, Self::Error>;

    /// open receiver stream for `endpoint`
    async fn open_write_stream(
        &mut self,
        endpoint: SyncEndpoint,
        mns: MNSAccount,
        open_stream: OpenStream,
    ) -> Result<Self::WriterStream, Self::Error>;
}

/// Endpoint sync status.
pub struct SyncStatus {
    pub send_seq_id: u64,
    pub recv_seq_id: u64,
}
