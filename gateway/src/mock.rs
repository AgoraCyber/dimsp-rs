use dimsp_types::{
    keccack256, open_write_stream_ack, sync_message, write_fragment_ack, CloseWriteStream, Hash32,
    MNSAccount, OpenWriteStream, SyncError, SyncMessage, WriteFragment,
};
use futures::{
    channel::mpsc::{self, SendError},
    stream::BoxStream,
    Future, SinkExt, StreamExt,
};
use protobuf::MessageField;

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

pub struct MockSession {
    conn: DatagramConnection<MockDatagramContext>,
}

impl MockSession {
    /// Send message
    pub async fn send_message<B: AsRef<[u8]>>(
        &mut self,
        buff: B,
        fragment_len: usize,
    ) -> anyhow::Result<()> {
        // prepare send messages
        let buff = buff.as_ref();

        let split_count = if buff.len() % fragment_len == 0 {
            buff.len() / fragment_len
        } else {
            buff.len() / fragment_len + 1
        };

        let mut fragments = vec![];
        let mut fragment_hashes: Vec<Hash32> = vec![];

        for i in 1..=split_count {
            let buff = if i * fragment_len > buff.len() {
                &buff[(i - 1) * fragment_len..]
            } else {
                &buff[(i - 1) * fragment_len..i * fragment_len]
            };

            let mut write_fragment = WriteFragment::new();

            write_fragment.offset = (i - 1) as u64;
            write_fragment.content = buff.to_vec();

            fragments.push(write_fragment);
            fragment_hashes.push(keccack256(buff).into());
        }

        log::debug!("Open write stream, fragments({})", fragments.len());

        let mut message = SyncMessage::new();
        message.type_ = sync_message::Type::OpenWriteStream.into();
        message.id = 1;

        let mut content = OpenWriteStream::new();

        content.length = buff.len() as u64;
        content.offset = 0;
        content.fragment_hashes = fragment_hashes;

        if fragments.len() == 1 {
            content.inline_stream = MessageField::from_option(fragments.pop());
        }

        message.set_open_write_stream(content);

        self.conn.send(message).await?;
        let ack = self.recv_ack(1).await?;

        log::debug!("Open write stream, ack: {}", ack);

        assert_eq!(ack.type_, sync_message::Type::OpenWriteStreamAck.into());

        if fragments.is_empty() {
            assert_eq!(
                ack.open_write_stream_ack().ack_type,
                open_write_stream_ack::Type::Noneed.into()
            );
        } else {
            assert_eq!(
                ack.open_write_stream_ack().ack_type,
                open_write_stream_ack::Type::Accept.into()
            );
        }

        let next_fragment = ack.open_write_stream_ack().next_fragment;
        let stream_handle = ack.open_write_stream_ack().stream_handle;

        let max_index = fragments.len();

        for (index, mut fragment) in fragments.into_iter().enumerate() {
            if index < next_fragment as usize {
                continue;
            }

            fragment.stream_handle = stream_handle;
            fragment.offset = index as u64;

            log::debug!("Write fragment({}) {}", index, fragment);

            let mut message = SyncMessage::new();
            message.type_ = sync_message::Type::WriteFragment.into();
            message.id = 1 + index as u64;
            message.set_write_fragment(fragment);

            self.conn.send(message).await?;

            let ack = self.recv_ack(1 + index as u64).await?;

            assert_eq!(ack.type_, sync_message::Type::WriteFragmentAck.into());

            if index + 1 == max_index {
                assert_eq!(
                    ack.write_fragment_ack().ack_type,
                    write_fragment_ack::Type::Nomore.into()
                );
            } else {
                assert_eq!(
                    ack.write_fragment_ack().ack_type,
                    write_fragment_ack::Type::Continue.into()
                );
            }

            log::debug!("Write fragment({}), ack: {}", index as u64, ack);
        }

        log::debug!("Close write stream");

        let mut content = CloseWriteStream::new();

        content.stream_handle = stream_handle;

        let mut message = SyncMessage::new();
        message.type_ = sync_message::Type::CloseWriteStream.into();
        message.id = (1 + max_index) as u64;
        message.set_close_write_stream(content);

        self.conn.send(message).await?;

        let ack = self.recv_ack(1 + max_index as u64).await?;

        assert_eq!(ack.type_, sync_message::Type::CloseWriteStreamAck.into());
        assert_eq!(
            ack.close_write_stream_ack().sync_error,
            SyncError::Success.into()
        );

        Ok(())
    }
    async fn recv_ack(&mut self, id: u64) -> anyhow::Result<SyncMessage> {
        let msg = self
            .conn
            .try_next()
            .await?
            .ok_or(anyhow::format_err!("broken piple"))?;

        log::debug!("msg id {}, expect {}", msg.id, id);

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
