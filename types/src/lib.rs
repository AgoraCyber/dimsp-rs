mod proto;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub use proto::sync::*;

mod mns;
pub use mns::*;

mod spss;
use protobuf::MessageField;
pub use spss::*;

mod uns;
pub use uns::*;

mod pubkey;
pub use pubkey::*;

mod endpoint;
pub use endpoint::*;

mod hash;
pub use hash::*;

/// Helper structure to build [`SyncMessage`]
pub struct SyncMessageBuilder(u64);

impl SyncMessageBuilder {
    /// Build new message, `ID` is a u64 data generator or is a simple u64 value.
    pub fn build<ID: Into<u64>>(id: ID) -> Self {
        Self(id.into())
    }

    pub fn open_write_stream(
        self,
        length: u64,
        to: u64,
        offset: u64,
        fragment_hashes: Vec<Hash32>,
        inline_stream: Option<WriteFragment>,
    ) -> SyncMessage {
        let mut content = OpenWriteStream::new();

        content.length = length;
        content.to = to;
        content.offset = offset;
        content.fragment_hashes = fragment_hashes;

        content.inline_stream = MessageField::from_option(inline_stream);

        let mut message = SyncMessage::new();

        message.type_ = sync_message::Type::OpenWriteStream.into();
        message.set_open_write_stream(content);
        message.id = self.0;

        message
    }

    pub fn close_write_stream(self, stream_handle: u64) -> SyncMessage {
        let mut close_write_stream = CloseWriteStream::new();

        close_write_stream.stream_handle = stream_handle;

        let mut message = SyncMessage::new();

        message.type_ = sync_message::Type::CloseWriteStream.into();
        message.set_close_write_stream(close_write_stream);
        message.id = self.0;

        message
    }

    pub fn write_fragment(self, stream_handle: u64, offset: u64, content: Vec<u8>) -> SyncMessage {
        let mut write_fragment = WriteFragment::new();

        write_fragment.stream_handle = stream_handle;
        write_fragment.offset = offset;
        write_fragment.content = content;

        let mut message = SyncMessage::new();

        message.type_ = sync_message::Type::WriteFragment.into();
        message.set_write_fragment(write_fragment);
        message.id = self.0;

        message
    }

    pub fn from_write_fragment(self, write_fragment: WriteFragment) -> SyncMessage {
        let mut message = SyncMessage::new();

        message.type_ = sync_message::Type::WriteFragment.into();
        message.set_write_fragment(write_fragment);
        message.id = self.0;

        message
    }

    pub fn open_inbox(self) -> SyncMessage {
        let mut message = SyncMessage::new();

        message.type_ = sync_message::Type::OpenInbox.into();

        message.id = self.0;

        message
    }

    pub fn open_next_inbox_stream(self) -> SyncMessage {
        unimplemented!()
    }
}

#[derive(Default)]
pub struct IdGenerator(Arc<AtomicU64>);

impl Into<u64> for &mut IdGenerator {
    fn into(self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}
