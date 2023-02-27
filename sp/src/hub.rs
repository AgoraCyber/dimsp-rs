use std::sync::{Arc, Mutex};

use dimsp_types::{sync_message::Type, SyncMessage};
use futures::{
    future::RemoteHandle,
    task::{SpawnError, SpawnExt},
    Future, SinkExt, TryStreamExt,
};
use once_cell::sync::OnceCell;

use thiserror::Error;

use crate::{
    gateway::{Connection, Gateway},
    sp_network::SpNetwork,
    storage::Storage,
};

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
    network: N,
    storage: S,
    wait_handler: Arc<Mutex<Option<RemoteHandle<()>>>>,
}

impl<G, N, S> Default for DimspHub<G, N, S>
where
    G: Gateway + Default,
    N: SpNetwork + Default,
    S: Storage + Default,
{
    fn default() -> Self {
        Self::new(Default::default(), Default::default(), Default::default())
    }
}

impl<G, N, S> From<(G, N, S)> for DimspHub<G, N, S>
where
    G: Gateway,
    N: SpNetwork,
    S: Storage,
{
    fn from(value: (G, N, S)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl<G, N, S> DimspHub<G, N, S>
where
    G: Gateway,
    N: SpNetwork,
    S: Storage,
{
    /// Create new DimspHub instance from ([`gateway`](Gateway),[`network`](SpNetwork),[`stoorage`](Storage))
    pub fn new(gateway: G, network: N, storage: S) -> Self {
        Self {
            gateway,
            network,
            storage,
            wait_handler: Default::default(),
        }
    }
}
impl<G, N, S> DimspHub<G, N, S>
where
    G: Gateway + Clone + Send + Sync + 'static,
    N: SpNetwork + Clone + Send + Sync + 'static,
    S: Storage + Clone + Send + Sync + 'static,
{
    /// Start [`DimspHub`] main event loop in background thread.
    pub fn start(&mut self) -> anyhow::Result<()> {
        let mut wait_handler = self.wait_handler.lock().unwrap();
        if wait_handler.is_none() {
            let hub = self.clone();
            let handle = Self::run_background(async move {
                match hub.event_loop().await {
                    Ok(_) => {
                        log::debug!("Dimsp event loop exit(SUCCESS).");
                    }
                    Err(err) => {
                        log::error!("Dimsp event loop exit(FAILED),{}.", err);
                    }
                }
            })?;

            *wait_handler = Some(handle);

            Ok(())
        } else {
            Err(DismpError::StartTwice.into())
        }
    }

    fn run_background<Fut>(fut: Fut) -> anyhow::Result<RemoteHandle<Fut::Output>>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        use futures::executor::ThreadPool;

        static THREAD_POOL: OnceCell<ThreadPool> = OnceCell::new();

        Ok(THREAD_POOL
            .get_or_init(|| ThreadPool::new().unwrap())
            .spawn_with_handle(fut)?)
    }
}

impl<G, N, S> DimspHub<G, N, S>
where
    G: Gateway + Clone + Send + Sync + 'static,
    N: SpNetwork + Clone + Send + Sync + 'static,
    S: Storage + Clone + Send + Sync + 'static,
{
    /// [`DimspHub`] main event loop function.
    async fn event_loop(mut self) -> anyhow::Result<()> {
        loop {
            let conn = self.gateway.accept().await?;

            let hub = self.clone();

            _ = Self::run_background(async move {
                let conn_id = conn.conn_id;
                let uns_id = conn.uns_id;
                match hub.handle_incoming_connection(conn).await {
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
    }
}

impl<G, N, S> DimspHub<G, N, S>
where
    G: Gateway + Clone + Send + 'static,
    N: SpNetwork + Clone + Send + 'static,
    S: Storage + Clone + Send + 'static,
{
    /// Handle incoming user connection.
    async fn handle_incoming_connection(
        mut self,
        mut conn: Connection<G::Input, G::Output, G::Error>,
    ) -> anyhow::Result<()> {
        use protobuf::Enum;

        while let Some(message) = conn.input.try_next().await? {
            // extract message type first.
            let message_type = Type::from_i32(message.type_.value())
                .ok_or(DismpError::SyncMessageType(message.type_.value()))?;

            let message_id = message.id;

            let mut response = match message_type {
                Type::OpenWriteStream => self.open_write_stream(conn.uns_id, message).await?,
                Type::CloseWriteStream => self.close_write_stream(conn.uns_id, message).await?,
                Type::OpenInbox => self.open_inbox(conn.uns_id, message).await?,
                Type::OpenNextInboxStream => {
                    self.open_next_inbox_stream(conn.uns_id, message).await?
                }
                Type::CloseInboxStream => self.close_inbox_stream(conn.uns_id, message).await?,
                Type::ReadFragment => self.read_fragment(conn.uns_id, message).await?,
                Type::WriteFragment => self.write_fragment(conn.uns_id, message).await?,
                _ => {
                    return Err(DismpError::SyncMessageType(message.type_.value()).into());
                }
            };

            response.id = message_id;

            conn.output.send(response).await?;
        }

        Ok(())
    }

    async fn open_write_stream(
        &mut self,
        uns_id: u64,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        if !message.has_open_write_stream() {
            return Err(DismpError::SyncMessageContent("OpenWriteStream".to_owned()).into());
        }

        let mns = self.network.mns_by_uns_id(uns_id).await?;

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
        _uns_id: u64,
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
        uns_id: u64,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        let mns = self.network.mns_by_uns_id(uns_id).await?;

        let ack = self.storage.open_inbox(mns).await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::OpenInboxAck.into();

        response.set_inbox(ack);

        Ok(response)
    }

    async fn open_next_inbox_stream(
        &mut self,
        uns_id: u64,
        message: SyncMessage,
    ) -> anyhow::Result<SyncMessage> {
        let mns = self.network.mns_by_uns_id(uns_id).await?;
        let ack = self.storage.open_next_inbox_stream(mns).await?;

        let mut response = SyncMessage::new();

        response.id = message.id;

        response.type_ = Type::OpenNextInboxStreamAck.into();

        response.set_open_next_inbox_stream_ack(ack);

        Ok(response)
    }

    async fn close_inbox_stream(
        &mut self,
        _uns_id: u64,
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
        _uns_id: u64,
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
        _uns_id: u64,
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