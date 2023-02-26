use agora_mail_types::{MNSAccount, OpenWriteStream, WriteFragment};
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Quotas: account don't have enough quota to write stream, required extra {0} bytes")]
    Quotas(usize),
}

/// Sync stream with persistence support.
#[async_trait]
pub trait Storage {
    type Error: std::error::Error + Sync + Send + 'static;

    type WriteStream: StorageWriteStream;

    /// Open write stream for [`mns`](MNSAccount) account.
    ///
    /// The implementation needs to determine whether the data is
    /// duplicated by the OpenWriteStream parameter and support
    /// fast uploading by returning None.
    async fn open_write_stream(
        &mut self,
        mns: MNSAccount,
        open_write_stream: OpenWriteStream,
    ) -> Result<Option<Self::WriteStream>, StorageError>;
}

#[async_trait]
pub trait StorageWriteStream {
    /// Write fragment
    async fn write_fragment(&mut self, fragment: WriteFragment) -> Result<(), StorageError>;

    /// Close write stream.
    async fn close(&mut self) -> Result<(), StorageError>;
}
