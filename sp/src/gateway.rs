use dimsp_types::SyncMessage;
use futures::{executor::block_on, Sink, SinkExt, Stream};

/// The abstract of client calling gateway.
#[async_trait::async_trait]
pub trait Gateway {
    type Error: std::error::Error + Sync + Send + 'static;

    type Input: Stream<Item = SyncMessage> + Send + Sync + Unpin;

    type Output: Sink<SyncMessage, Error = Self::Error> + Send + Sync + Unpin;

    /// Accept one incoming transport layer connection.
    /// The gateway must check the valid status of the access user,
    /// invalid status user connection should be closed directly.
    async fn accept(&mut self) -> anyhow::Result<Option<Connection<Self::Input, Self::Output>>>;
}

/// Effective user connection
pub struct Connection<Input, Output>
where
    Input: Stream<Item = SyncMessage> + Send + Sync + Unpin,
    Output: Sink<SyncMessage> + Send + Sync + Unpin,
    Output::Error: std::error::Error + Sync + Send + 'static,
{
    /// In-process connection sequence id
    pub conn_id: u64,
    /// UNS(User name service) NFT id.
    pub uns_id: u64,
    /// stream storage lease duration
    /// Input stream,
    pub input: Input,
    /// Output stream.
    pub output: Output,
}

impl<Input, Output> Drop for Connection<Input, Output>
where
    Input: Stream<Item = SyncMessage> + Send + Sync + Unpin,
    Output: Sink<SyncMessage> + Send + Sync + Unpin,
    Output::Error: std::error::Error + Sync + Send + 'static,
{
    fn drop(&mut self) {
        match block_on(self.output.close()) {
            Err(err) => {
                log::error!("Close gateway connection failed, {:?}", err);
            }
            Ok(_) => {}
        }
    }
}
