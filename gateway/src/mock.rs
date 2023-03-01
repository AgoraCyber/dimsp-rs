use dimsp_types::{
    keccack256, sync_message, MNSAccount, OpenWriteStream, SyncMessage, WriteFragment,
};
use futures::{
    channel::mpsc::{self, SendError},
    stream::BoxStream,
    Future, SinkExt, StreamExt,
};
use protobuf::MessageField;

use crate::{DatagramConnection, DatagramContext, DatagramGateway};

const MAX_FRAGMENT_LEN: usize = 1024 * 1024 * 4;

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

pub struct MockSession {
    conn: DatagramConnection<MockDatagramContext>,
}

impl MockSession {
    /// Send message
    pub async fn send_message<B: AsRef<[u8]>>(&mut self, buff: B) -> anyhow::Result<()> {
        // prepare send messages
        let buff = buff.as_ref();

        let split_count = if buff.len() % MAX_FRAGMENT_LEN == 0 {
            buff.len() / MAX_FRAGMENT_LEN
        } else {
            buff.len() / MAX_FRAGMENT_LEN + 1
        };

        let mut fragments = vec![];

        for i in 1..=split_count {
            let buff = if i * MAX_FRAGMENT_LEN > buff.len() {
                &buff[(i - 1) * MAX_FRAGMENT_LEN..]
            } else {
                &buff[(i - 1) * MAX_FRAGMENT_LEN..i * MAX_FRAGMENT_LEN]
            };

            let mut write_fragment = WriteFragment::new();

            write_fragment.offset = (i - 1) as u64;
            write_fragment.content = buff.to_vec();

            fragments.push(write_fragment);
        }

        let mut message = SyncMessage::new();
        message.type_ = sync_message::Type::OpenWriteStream.into();
        message.id = 1;

        let mut content = OpenWriteStream::new();

        content.length = buff.len() as u64;
        content.offset = 0;
        content.fragment_hashes = vec![keccack256(buff).into()];

        if fragments.len() == 0 {
            content.inline_stream = MessageField::from_option(fragments.pop());
        }

        message.set_open_write_stream(content);

        self.conn.send(message).await?;
        let ack = self.recv_ack(1).await?;

        log::debug!("recv {}", ack);

        Ok(())
    }
    async fn recv_ack(&mut self, id: u64) -> anyhow::Result<SyncMessage> {
        let msg = self
            .conn
            .try_next()
            .await?
            .ok_or(anyhow::format_err!("broken piple"))?;

        if msg.id != id {
            return Err(anyhow::format_err!("ack seq id mismatch !!!"));
        }

        Ok(msg)
    }
}

pub struct MockClient {
    seq: usize,
    sender: mpsc::Sender<DatagramConnection<MockDatagramContext>>,
}

impl MockClient {
    pub async fn connect_with(&mut self, mns: MNSAccount) -> anyhow::Result<MockSession> {
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

        Ok(MockSession { conn: client_conn })
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
