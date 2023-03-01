use dimsp_types::{MNSAccount, SyncMessage};
use futures::{Future, Sink, SinkExt, Stream, TryStreamExt};

/// Datagram connection implementation
pub trait DatagramContext {
    type StreamError: std::error::Error + Send + Sync + 'static;
    type SinkError: std::error::Error + Send + Sync + 'static;
    type Item;
    type Context;
    type Input: Stream<Item = Result<Self::Item, Self::StreamError>> + Send + Unpin;
    type Output: Sink<Self::Item, Error = Self::SinkError> + Send + Unpin;
}

/// Datagram connection wrap structure.
#[derive(Debug)]
pub struct DatagramConnection<C: DatagramContext> {
    /// Connection global sequence id.
    pub id: usize,
    /// Connection bind context.
    pub context: C::Context,
    /// Connection input stream.
    pub input: C::Input,
    /// Connection output stream.
    pub output: C::Output,
}

impl<C: DatagramContext> DatagramConnection<C> {
    /// Wrapper for output stream send method
    pub async fn send(&mut self, item: C::Item) -> Result<(), C::SinkError> {
        self.output.send(item).await
    }

    /// Wrapper for input stream try_next method
    pub async fn try_next(&mut self) -> Result<Option<C::Item>, C::StreamError> {
        self.input.try_next().await
    }
}

/// Datgram transport gateway.
pub trait DatagramGateway {
    /// Gateway association connection [`context`](DatagramContext).
    type Context: DatagramContext<Item = SyncMessage, Context = MNSAccount>;

    /// [`accept`] function returns future.
    type Accepable<'cx>: Future<Output = Option<DatagramConnection<Self::Context>>> + Send + 'cx
    where
        Self: 'cx;

    /// Accept one incoming connection, or returns [`None`] if gateway shutdown.
    fn accept<'a, 'cx>(&'a mut self) -> Self::Accepable<'cx>
    where
        'a: 'cx;
}
