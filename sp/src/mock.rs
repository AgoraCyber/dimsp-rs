use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::sp_network::SpNetwork;

use async_trait::async_trait;

use dimsp_storage::Storage;
use dimsp_types::*;

use thiserror::Error;

#[derive(Debug, Error)]
enum MockError {}

#[derive(Default)]
struct MockSpNetworkImpl {
    subscribed_by: HashMap<u64, Vec<SPRSAccount>>,
    mns_by_pubkey: HashMap<PublicKey, MNSAccount>,
    mns_by_id: HashMap<u64, MNSAccount>,
    sprs: HashMap<u64, SPRSAccount>,
}

impl Drop for MockSpNetworkImpl {
    fn drop(&mut self) {
        log::debug!("drop sp network");
    }
}

#[derive(Default, Clone)]
pub struct MockSpNetwork {
    inner: Arc<Mutex<MockSpNetworkImpl>>,
}

impl MockSpNetwork {
    pub fn add_subscribed(&mut self, mns_id: u64, subscriptions: &[SPRSAccount]) {
        let mut inner = self.inner.lock().unwrap();

        inner.subscribed_by.insert(mns_id, subscriptions.to_owned());

        for sprs in subscriptions {
            inner.sprs.insert(sprs.mns.uns.id, sprs.clone());
        }
    }

    pub fn add_mns(&mut self, mns: MNSAccount) {
        let mut inner = self.inner.lock().unwrap();
        inner.mns_by_pubkey.insert(mns.pub_key.clone(), mns.clone());
        inner.mns_by_id.insert(mns.uns.id, mns);
    }
}

#[async_trait]
impl SpNetwork for MockSpNetwork {
    async fn sps_subscribed_by(&self, mns: MNSAccount) -> anyhow::Result<Option<Vec<SPRSAccount>>> {
        let inner = self.inner.lock().unwrap();

        Ok(inner.subscribed_by.get(&mns.uns.id).map(|c| c.clone()))
    }

    /// Search mns account by public key. maybe returns [`None`]
    async fn mns_by_pubkey(&self, pubkey: PublicKey) -> anyhow::Result<Option<MNSAccount>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.mns_by_pubkey.get(&pubkey).map(Clone::clone))
    }

    /// Get mns account by **uns** id.
    async fn mns_by_uns_id(&self, uns_id: u64) -> anyhow::Result<Option<MNSAccount>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.mns_by_id.get(&uns_id).map(Clone::clone))
    }

    /// Get sprs account by **uns** id.
    async fn sp_by_uns_id(&self, uns_id: u64) -> anyhow::Result<Option<SPRSAccount>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.sprs.get(&uns_id).map(Clone::clone))
    }
}

#[derive(Clone)]
struct Blob {
    stream_id: u64,
    length: u64,
    account: u64,
    fragment_hashes: Vec<Hash32>,
}

#[derive(Default)]
struct MockStorageImpl {
    memory: HashMap<u64, Vec<Vec<u8>>>,
    timeline: HashMap<u64, Vec<Blob>>,
    blob_list: HashMap<u64, Blob>,
    seq: u64,
}

impl Drop for MockStorageImpl {
    fn drop(&mut self) {
        log::debug!("drop storage");
    }
}

#[derive(Default, Clone)]
pub struct MockStorage {
    inner: Arc<Mutex<MockStorageImpl>>,
}

#[async_trait]
impl Storage for MockStorage {
    async fn open_write_stream(
        &mut self,
        mns: MNSAccount,
        open_write_stream: OpenWriteStream,
    ) -> anyhow::Result<OpenWriteStreamAck> {
        let mut inner = self.inner.lock().unwrap();

        let stream_id = inner.seq;

        inner.seq += 1;

        let blob = Blob {
            stream_id,
            account: mns.uns.id,
            length: open_write_stream.length,
            fragment_hashes: open_write_stream.fragment_hashes,
        };

        inner.memory.insert(mns.uns.id, vec![]);

        inner.blob_list.insert(stream_id, blob);

        let mut ack = OpenWriteStreamAck::new();

        ack.ack_type = open_write_stream_ack::Type::Accept.into();

        Ok(ack)
    }

    /// Write new fragment
    async fn write_fragment(
        &mut self,
        write_fragment: WriteFragment,
    ) -> anyhow::Result<WriteFragmentAck> {
        let mut inner = self.inner.lock().unwrap();

        let blob = inner.blob_list.get(&write_fragment.stream_handle);

        if blob.is_none() {
            let mut ack = WriteFragmentAck::new();

            ack.ack_type = write_fragment_ack::Type::Break.into();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap().clone();

        inner
            .memory
            .get_mut(&blob.stream_id)
            .unwrap()
            .push(write_fragment.content);

        let mut ack = WriteFragmentAck::new();

        ack.ack_type = write_fragment_ack::Type::Continue.into();

        return Ok(ack);
    }

    /// Close write stream.
    async fn close_write_stream(
        &mut self,
        close_write_stream: CloseWriteStream,
    ) -> anyhow::Result<CloseWriteStreamAck> {
        let mut inner = self.inner.lock().unwrap();

        let blob = inner.blob_list.remove(&close_write_stream.stream_handle);

        if blob.is_none() {
            let mut ack = CloseWriteStreamAck::new();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        inner
            .timeline
            .entry(blob.account)
            .and_modify(|c| c.push(blob.clone()))
            .or_insert(vec![blob]);

        let ack = CloseWriteStreamAck::new();

        return Ok(ack);
    }

    /// Open next inbox read stream with [`mns`](MNSAccount)
    async fn open_next_inbox_stream(
        &mut self,
        mns: MNSAccount,
    ) -> anyhow::Result<OpenNextInboxStreamAck> {
        let mut inner = self.inner.lock().unwrap();

        let time_line = inner.timeline.get(&mns.uns.id);

        if time_line.is_none() {
            let mut ack = OpenNextInboxStreamAck::new();

            ack.type_ = open_next_inbox_stream_ack::Type::Nomore.into();

            return Ok(ack);
        }

        let blob = time_line.unwrap().first();

        if blob.is_none() {
            let mut ack = OpenNextInboxStreamAck::new();

            ack.type_ = open_next_inbox_stream_ack::Type::Nomore.into();

            return Ok(ack);
        }

        let blob = blob.unwrap().clone();

        inner.blob_list.insert(blob.stream_id, blob.clone());

        let mut open_read_stream = OpenReadStream::new();

        open_read_stream.stream_handle = blob.stream_id;
        open_read_stream.length = blob.length;
        open_read_stream.fragment_hashes = blob.fragment_hashes.clone();

        let mut ack = OpenNextInboxStreamAck::new();

        ack.type_ = open_next_inbox_stream_ack::Type::Accept.into();

        use ::protobuf::MessageField;

        ack.read_stream = MessageField::some(open_read_stream);

        Ok(ack)
    }

    /// Read fragment
    async fn read_fragment(
        &mut self,
        read_fragment: ReadFragment,
    ) -> anyhow::Result<ReadFragmentAck> {
        let inner = self.inner.lock().unwrap();

        let stream = inner.memory.get(&read_fragment.stream_handle);

        if stream.is_none() {
            let mut ack = ReadFragmentAck::new();

            ack.ack_type = read_fragment_ack::Type::Break.into();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let stream = stream.unwrap();

        if (stream.len() as u64) < read_fragment.offset {
            let mut ack = ReadFragmentAck::new();

            ack.ack_type = read_fragment_ack::Type::Break.into();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let data = stream[read_fragment.offset as usize].clone();

        let mut ack = ReadFragmentAck::new();

        ack.ack_type = read_fragment_ack::Type::Continue.into();

        ack.content = data;

        Ok(ack)
    }

    /// Close read stream.
    async fn close_inbox_stream(
        &mut self,
        close_inbox_stream: CloseInboxStream,
    ) -> anyhow::Result<CloseInboxStreamAck> {
        let mut inner = self.inner.lock().unwrap();

        let blob = inner.blob_list.remove(&close_inbox_stream.stream_handle);

        if blob.is_none() {
            let mut ack = CloseInboxStreamAck::new();

            ack.stream_handle = close_inbox_stream.stream_handle;

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        inner
            .timeline
            .entry(blob.account)
            .and_modify(|c| c.push(blob.clone()))
            .or_insert(vec![blob]);

        let mut ack = CloseInboxStreamAck::new();

        ack.stream_handle = close_inbox_stream.stream_handle;

        return Ok(ack);
    }

    /// Read inbox information of [`mns`](MNSAccount)
    async fn open_inbox(&mut self, mns: MNSAccount) -> anyhow::Result<Inbox> {
        let inner = self.inner.lock().unwrap();
        let time_line = inner.timeline.get(&mns.uns.id);

        if time_line.is_none() {
            let inbox = Inbox::new();

            return Ok(inbox);
        }

        let time_line = time_line.unwrap();

        let mut total_length = 0;

        for blob in time_line {
            total_length += blob.length;
        }

        let mut inbox = Inbox::new();

        inbox.total_length = total_length;
        inbox.unread = time_line.len() as u64;

        Ok(inbox)
    }
}
