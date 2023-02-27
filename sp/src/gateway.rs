use std::fmt::Debug;

use dimsp_types::SyncMessage;
use futures::{executor::block_on, Sink, SinkExt, Stream};

/// The abstract of client calling gateway.
#[async_trait::async_trait]
pub trait Gateway {
    type Error: std::error::Error + Debug;

    type Input: Stream<Item = Result<SyncMessage, Self::Error>> + Send + Sync + Unpin;

    type Output: Sink<SyncMessage, Error = Self::Error> + Send + Sync + Unpin;

    /// Accept one incoming transport layer connection.
    /// The gateway must check the valid status of the access user,
    /// invalid status user connection should be closed directly.
    async fn accept(
        &mut self,
    ) -> Result<Connection<Self::Input, Self::Output, Self::Error>, Self::Error>;
}

/// Effective user connection
pub struct Connection<Input, Output, Error>
where
    Input: Stream<Item = Result<SyncMessage, Error>> + Send + Sync + Unpin,
    Output: Sink<SyncMessage, Error = Error> + Send + Sync + Unpin,
    Error: std::error::Error + Debug,
{
    /// In-process connection sequence id
    pub conn_id: u64,
    /// UNS(User name service) NFT id.
    pub uns_id: String,
    /// Input stream,
    pub input: Input,
    /// Output stream.
    pub output: Output,
}

impl<Input, Output, Error> Drop for Connection<Input, Output, Error>
where
    Input: Stream<Item = Result<SyncMessage, Error>> + Send + Sync + Unpin,
    Output: Sink<SyncMessage, Error = Error> + Send + Sync + Unpin,
    Error: std::error::Error + Debug,
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
