use dimsp_types::{MNSAccount, SyncMessage};
use futures::{
    channel::mpsc::{self, SendError},
    stream::BoxStream,
    Future, SinkExt, StreamExt,
};

use crate::{DatagramConnection, DatagramContext, DatagramGateway};

pub struct MockDatagramContext;

#[derive(Debug, thiserror::Error)]
pub enum MockError {}

impl DatagramContext for MockDatagramContext {
    type SinkError = SendError;
    type StreamError = MockError;
    type Item = SyncMessage;
    type Context = MNSAccount;
    type Input = BoxStream<'static, Result<SyncMessage, Self::StreamError>>;
    type Output = mpsc::Sender<SyncMessage>;
}

pub struct MockClient {
    seq: usize,
    sender: mpsc::Sender<DatagramConnection<MockDatagramContext>>,
}

impl MockClient {
    pub async fn connect_with(
        &mut self,
        mns: MNSAccount,
    ) -> anyhow::Result<DatagramConnection<MockDatagramContext>> {
        let (server_sender, client_receiver) = mpsc::channel(100);
        let (client_sender, server_receiver) = mpsc::channel(100);

        self.seq += 1;

        let server_conn = DatagramConnection {
            id: self.seq,
            context: mns.clone(),
            input: server_receiver.map(|c| Ok(c)).boxed(),
            output: server_sender,
        };

        let client_conn = DatagramConnection {
            id: self.seq,
            context: mns,
            input: client_receiver.map(|c| Ok(c)).boxed(),
            output: client_sender,
        };

        self.sender.send(server_conn).await?;

        Ok(client_conn)
    }
}

pub struct MockGateway {
    receiver: mpsc::Receiver<DatagramConnection<MockDatagramContext>>,
}

impl MockGateway {
    pub fn new() -> (MockGateway, MockClient) {
        let (sender, receiver) = mpsc::channel(100);

        (Self { receiver }, MockClient { seq: 0, sender })
    }
}

impl DatagramGateway for MockGateway {
    type Context = MockDatagramContext;

    type Accepable<'cx> = MockAccepable<'cx>;

    fn accept<'a, 'cx>(&'a mut self) -> Self::Accepable<'cx>
    where
        'a: 'cx,
    {
        MockAccepable {
            receiver: &mut self.receiver,
        }
    }
}

pub struct MockAccepable<'cx> {
    receiver: &'cx mut mpsc::Receiver<DatagramConnection<MockDatagramContext>>,
}

impl<'cx> Future for MockAccepable<'cx> {
    type Output = Option<DatagramConnection<MockDatagramContext>>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.receiver.poll_next_unpin(cx)
    }
}
