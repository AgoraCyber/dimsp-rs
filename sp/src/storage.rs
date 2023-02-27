use async_trait::async_trait;
use dimsp_types::{
    CloseInboxStream, CloseInboxStreamAck, CloseWriteStream, CloseWriteStreamAck, Inbox,
    MNSAccount, OpenNextInboxStreamAck, OpenWriteStream, OpenWriteStreamAck, ReadFragment,
    ReadFragmentAck, WriteFragment, WriteFragmentAck,
};

/// Storage implementation standard errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Returns if the account don't have enough quota
    #[error(
        "Quotas: the account don't have enough quota to write stream, required extra {0} bytes."
    )]
    QuotasError(usize),
    /// Returns if the account reach the end of the inbox reading
    #[error("Nomore: the mns account reach the end of the inbox reading.")]
    NomoreError(MNSAccount),
}

/// Sync stream with persistence support.
#[async_trait]
pub trait Storage {
    /// Open write stream for [`mns`](MNSAccount) account.
    ///
    /// The implementation needs to determine whether the data is
    /// duplicated by the OpenWriteStream parameter and support
    /// fast uploading by returning None.
    async fn open_write_stream(
        &mut self,
        mns: MNSAccount,
        open_write_stream: OpenWriteStream,
    ) -> anyhow::Result<OpenWriteStreamAck>;

    /// Write new fragment
    async fn write_fragment(
        &mut self,
        write_fragment: WriteFragment,
    ) -> anyhow::Result<WriteFragmentAck>;

    /// Close write stream.
    async fn close_write_stream(
        &mut self,
        close_write_stream: CloseWriteStream,
    ) -> anyhow::Result<CloseWriteStreamAck>;

    /// Open next inbox read stream with [`mns`](MNSAccount)
    async fn open_next_inbox_stream(
        &mut self,
        mns: MNSAccount,
    ) -> anyhow::Result<OpenNextInboxStreamAck>;

    /// Read fragment
    async fn read_fragment(
        &mut self,
        read_fragment: ReadFragment,
    ) -> anyhow::Result<ReadFragmentAck>;

    /// Close read stream.
    async fn close_inbox_stream(
        &mut self,
        close_inbox_stream: CloseInboxStream,
    ) -> anyhow::Result<CloseInboxStreamAck>;

    /// Read inbox information of [`mns`](MNSAccount)
    async fn open_inbox(&mut self, mns: MNSAccount) -> anyhow::Result<Inbox>;
}
