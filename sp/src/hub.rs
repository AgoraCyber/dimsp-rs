use dimsp_spnetwork::SpNetwork;
use dimsp_types::{sync_message::Type, MNSAccount, SyncMessage};
use futures::{task::SpawnError, SinkExt, TryStreamExt};

use thiserror::Error;

use crate::threadpool::run_background;

use dimsp_gateway::*;
use dimsp_storage::Storage;

#[derive(Debug, Error)]
pub enum DismpError {
    #[error("SpawnError: {0}")]
    SpwanError(#[from] SpawnError),
    #[error("StartTwice: call start method twice")]
    StartTwice,
    #[error("SyncMessageTypeError: DimspHub can'nt handle sync message type with id {0}")]
    SyncMessageType(i32),

    /// Read sync message content is none
    #[error("SyncMessageContent: SyncMessage content({0}) is none")]
    SyncMessageContent(String),
}

/// Dimsp service provider node implementation
#[derive(Debug, Clone)]
pub struct DimspHub<G, N, S> {
    gateway: G,
    #[allow(unused)]
    network: N,
    storage: S,
}

impl<G, N, S> Drop for DimspHub<G, N, S> {
    fn drop(&mut self) {
        log::debug!("drop hub");
    }
}

impl<G, N, S> Default for DimspHub<G, N, S>
where
    G: DatagramGateway + Default,
    N: SpNetwork + Default,
    S: Storage + Default,
{
    fn default() -> Self {
        Self::new(Default::default(), Default::default(), Default::default())
    }
}

impl<G, N, S> From<(G, N, S)> for DimspHub<G, N, S>
where
    G: DatagramGateway,
    N: SpNetwork,
    S: Storage,
{
    fn from(value: (G, N, S)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl<G, N, S> DimspHub<G, N, S>
where
    G: DatagramGateway,
    N: SpNetwork,
    S: Storage,
{
    /// Create new DimspHub instance from ([`gateway`](DatagramGateway),[`network`](SpNetwork),[`stoorage`](Storage))
    pub fn new(gateway: G, network: N, storage: S) -> Self {
        Self {
            gateway,
            network,
            storage,
        }
    }
}
impl<G, N, S> DimspHub<G, N, S>
where
    G: DatagramGateway + Send + Sync + 'static,
    N: SpNetwork + Clone + Send + Sync + 'static,
    S: Storage + Clone + Send + Sync + 'static,
{
    /// Start [`DimspHub`] main event loop in background thread.
    pub fn start(self) -> anyhow::Result<()> {
        let hub = self;

        run_background(async move {
            match hub.event_loop().await {
                Ok(_) => {
                    log::debug!("Dimsp event loop exit(SUCCESS).");
                }
                Err(err) => {
                    log::error!("Dimsp event loop exit(FAILED),{}.", err);
                }
            }
        })
    }
}

struct DimspHubSession<N, S> {
    #[allow(unused)]
    network: N,
    storage: S,
}

impl<G, N, S> DimspHub<G, N, S>
where
    G: DatagramGateway + Send + Sync + 'static,
    N: SpNetwork + Clone + Send + Sync + 'static,
    S: Storage + Clone + Send + Sync + 'static,
{
    fn new_session(&self) -> DimspHubSession<N, S> {
        DimspHubSession {
            network: self.network.clone(),
            storage: self.storage.clone(),
        }
    }

    /// [`DimspHub`] main event loop function.
    async fn event_loop(mut self) -> anyhow::Result<()> {
        while let Some(conn) = self.gateway.accept().await {
            log::debug!("loop {}", conn.id);
            let session = self.new_session();

            _ = run_background(async move {
                let conn_id = conn.id;
                let uns_id = conn.context.uns.id;
                match session.handle_incoming_connection::<G>(conn).await {
                    Ok(_) => {
                        log::info!("UNS({}) connection({}) closed", uns_id, conn_id);
                    }
                    Err(err) => {
                        log::info!(
                            "UNS({}) connection({}) closed with err, {}",
                            uns_id,
                            conn_id,
                            err
                        );
                    }
                }
            })?;
        }

        Ok(())
    }
}

impl<N, S> DimspHubSession<N, S>
where
    N: SpNetwork + Clone + Send + 'static,
    S: Storage + Clone + Send + 'static,
{
    /// Handle incoming user connection.
    async fn handle_incoming_connection<G: DatagramGateway + Send + Sync + 'static>(
        mut self,
        mut conn: DatagramConnection<G::Context>,
    ) -> anyhow::Result<()> {
        use protobuf::Enum;

        log::debug!(
            "handle user({}) connection({})",
            conn.context.uns.id,
            conn.id
        );

        while let Some(message) = conn.input.try_next().await? {
            // extract message type first.
            let message_type = Type::from_i32(message.type_.value())
                .ok_or(DismpError::SyncMessageType(message.type_.value()))?;

            let message_id = message.id;

            let mut response = match message_type {
                Type::OpenWriteStream => {
                    self.open_write_stream(conn.context.clone(), message)
                        .await?
                }
                Type::CloseWriteStream => {
                    self.close_write_stream(conn.context.clone(), message)
                        .await?
                }
                Type::OpenInbox => self.open_inbox(conn.context.clone(), message).await?,
                Type::OpenNextInboxStream => {
                    self.open_next_inbox_stream(conn.context.clone(), message)
                        .await?
                }
                Type::CloseInboxStream => {
                    self.close_inbox_stream(conn.context.clone(), message)
                        .await?
                }
                Type::ReadFragment => self.read_fragment(conn.context.clone(), message).await?,
                Type::WriteFragment => self.write_fragment(conn.context.clone(), message).await?,
                _ => {
                    return Err(DismpError::SyncMessageType(message.type_.value()).into());
                }
            };

            response.id = message_id;

            log::debug!("send response {}", message_id);

            conn.output.send(response).await?;
        }

        Ok(())
    }

    async fn open_write_stream(
        &mut self,
        mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_open_write_stream() {
            return Err(DismpError::SyncMessageContent("OpenWriteStream".to_owned()).into());
        }

        let ack = self
            .storage
            .open_write_stream(mns, message.open_write_stream().clone())
            .await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::OpenWriteStreamAck.into();

        response.set_open_write_stream_ack(ack);

        Ok(response)
    }

    async fn close_write_stream(
        &mut self,
        _mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_close_write_stream() {
            return Err(DismpError::SyncMessageContent("CloseWriteStream".to_owned()).into());
        }

        let ack = self
            .storage
            .close_write_stream(message.close_write_stream().clone())
            .await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::CloseWriteStreamAck.into();

        response.set_close_write_stream_ack(ack);

        Ok(response)
    }

    async fn open_inbox(
        &mut self,
        mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        let ack = self.storage.open_inbox(mns).await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::OpenInboxAck.into();

        response.set_inbox(ack);

        Ok(response)
    }

    async fn open_next_inbox_stream(
        &mut self,
        mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        let ack = self.storage.open_next_inbox_stream(mns).await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::OpenNextInboxStreamAck.into();

        response.set_open_next_inbox_stream_ack(ack);

        Ok(response)
    }

    async fn close_inbox_stream(
        &mut self,
        _mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_close_inbox_stream() {
            return Err(DismpError::SyncMessageContent("CloseInboxStream".to_owned()).into());
        }

        let ack = self
            .storage
            .close_inbox_stream(message.close_inbox_stream().clone())
            .await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::CloseInboxStreamAck.into();

        response.set_close_inbox_stream_ack(ack);

        Ok(response)
    }

    async fn read_fragment(
        &mut self,
        _mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_close_inbox_stream() {
            return Err(DismpError::SyncMessageContent("ReadFragment".to_owned()).into());
        }

        let ack = self
            .storage
            .read_fragment(message.read_fragment().clone())
            .await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::ReadFragmentAck.into();

        response.set_read_fragment_ack(ack);

        Ok(response)
    }

    async fn write_fragment(
        &mut self,
        _mns: MNSAccount,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_close_inbox_stream() {
            return Err(DismpError::SyncMessageContent("WriteFrament".to_owned()).into());
        }

        let ack = self
            .storage
            .write_fragment(message.write_fragment().clone())
            .await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::WriteFragmentAck.into();

        response.set_write_fragment_ack(ack);

        Ok(response)
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use std::time::Duration;

    use dimsp_gateway::mock::MockGateway;
    use dimsp_spnetwork::mock::MockSpNetwork;
    use dimsp_storage::mock::MockStorage;
    use dimsp_types::MNSAccount;

    use super::DimspHub;

    #[async_std::test]
    async fn test_write() {
        _ = pretty_env_logger::try_init();

        let (gateway, mut client) = MockGateway::new();

        let storage = MockStorage::default();

        let network = MockSpNetwork::default();

        let hub = DimspHub::new(gateway, network, storage);

        hub.start().unwrap();

        // Create mock account
        let mut account = MNSAccount::default();

        account.uns.id = 100;
        account.quota = 1024 * 1024 * 4;
        account.lease = Duration::from_secs(10);

        let mut session = client.connect_with(account).await.unwrap();

        session.send_message("Hello world").await.unwrap();
    }
}
