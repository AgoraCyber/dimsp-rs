use agora_mail_proto::sync::SyncMessage;
use futures::{Sink, Stream};

/// The abstract of client calling gateway.
#[async_trait::async_trait]
pub trait Gateway {
    type Error: std::error::Error;

    /// Accept one incoming request connection.
    /// The gateway must check the valid status of the access user,
    /// invalid status user connection should be closed directly.
    async fn accept(&mut self) -> Result<(), Self::Error>;
}

/// Effective user connection
pub struct Connection<Input, Output>
where
    Input: Stream<Item = SyncMessage> + Send + Sync,
    Output: Sink<SyncMessage> + Send + Sync,
{
    /// UNS(User name service) NFT id.
    pub uns_id: String,
    /// Input stream,
    pub input: Input,
    /// Output stream.
    pub output: Output,
}
