use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use dimsp_types::*;
use snowflake::SnowflakeIdGenerator;

use crate::{
    blob::{Blob, BlobProvider},
    timeline::TimelineProvider,
};

#[derive(Clone)]
pub struct DimspHubStorage<T, B> {
    /// Global sequence generator using snowflake alogrithem
    seq: Arc<Mutex<SnowflakeIdGenerator>>,
    /// opened blob_list
    blob_list: Arc<Mutex<HashMap<u64, Blob>>>,
    /// timeline provider
    timeline: T,
    /// blob provider
    blob: B,
}

impl<T, B> DimspHubStorage<T, B>
where
    T: TimelineProvider + Sync + Send + 'static,
    B: BlobProvider + Sync + Send + 'static,
{
    pub fn new(machine_id: i32, timeline: T, blob: B) -> Self {
        DimspHubStorage {
            seq: Arc::new(Mutex::new(SnowflakeIdGenerator::new(
                machine_id, machine_id,
            ))),
            blob_list: Default::default(),
            timeline,
            blob,
        }
    }
}

#[allow(unused)]
#[async_trait]
impl<T, B> Storage for DimspHubStorage<T, B>
where
    T: TimelineProvider + Sync + Send + 'static,
    B: BlobProvider + Sync + Send + 'static,
{
    /// Open write stream for [`mns`](MNSAccount) account.
    ///
    /// The implementation needs to determine whether the data is
    /// duplicated by the OpenWriteStream parameter and support
    /// fast uploading by returning None.
    async fn open_write_stream(
        &mut self,
        mns: MNSAccount,
        open_write_stream: OpenWriteStream,
    ) -> anyhow::Result<OpenWriteStreamAck> {
        let max_fragments = open_write_stream.fragment_hashes.len();

        let blob = self
            .blob
            .start_write_blob(
                &mns,
                open_write_stream.length,
                open_write_stream.fragment_hashes,
            )
            .await;

        let blob = match blob {
            Ok(blob) => blob,
            Err(err) => match err.downcast_ref() {
                Some(StorageError::QuotasError(quota)) => {
                    let mut ack = OpenWriteStreamAck::new();

                    ack.ack_type = open_write_stream_ack::Type::Reject.into();

                    ack.sync_error = SyncError::Quota.into();

                    return Ok(ack);
                }

                _ => return Err(err),
            },
        };

        let mut ack = OpenWriteStreamAck::new();

        if blob.next_fragment as usize == max_fragments {
            ack.ack_type = open_write_stream_ack::Type::Noneed.into();

            return Ok(ack);
        }

        let stream_handle = self.seq.lock().unwrap().real_time_generate() as u64;

        let next_fragment = blob.next_fragment;

        self.blob_list.lock().unwrap().insert(stream_handle, blob);

        ack.ack_type = open_write_stream_ack::Type::Accept.into();

        ack.next_fragment = next_fragment;

        ack.stream_handle = stream_handle;

        Ok(ack)
    }

    /// Write new fragment
    async fn write_fragment(
        &mut self,
        write_fragment: WriteFragment,
    ) -> anyhow::Result<WriteFragmentAck> {
        let blob = self
            .blob_list
            .lock()
            .unwrap()
            .get(&write_fragment.stream_handle)
            .map(|c| c.clone());

        if blob.is_none() {
            let mut ack = WriteFragmentAck::new();

            ack.ack_type = write_fragment_ack::Type::Break.into();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        self.blob
            .write_blob_fragment(&blob.id, write_fragment.offset, write_fragment.content)
            .await?;

        let mut ack = WriteFragmentAck::new();

        ack.ack_type = write_fragment_ack::Type::Continue.into();

        ack.stream_handle = write_fragment.stream_handle;
        ack.offset = write_fragment.offset;

        Ok(ack)
    }

    /// Close write stream.
    async fn close_write_stream(
        &mut self,
        close_write_stream: CloseWriteStream,
    ) -> anyhow::Result<CloseWriteStreamAck> {
        let blob = self
            .blob_list
            .lock()
            .unwrap()
            .remove(&close_write_stream.stream_handle);

        if blob.is_none() {
            let mut ack = CloseWriteStreamAck::new();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        self.blob.end_write_blob(&blob.id).await?;

        self.timeline.append(blob).await?;

        let mut ack = CloseWriteStreamAck::new();

        return Ok(ack);
    }

    /// Open next inbox read stream with [`mns`](MNSAccount)
    async fn open_next_inbox_stream(
        &mut self,
        mns: MNSAccount,
    ) -> anyhow::Result<OpenNextInboxStreamAck> {
        if let Some(blob) = self.timeline.get(&mns).await? {
            let stream_handle = self.seq.lock().unwrap().real_time_generate() as u64;

            let mut open_read_stream = OpenReadStream::new();

            open_read_stream.stream_handle = stream_handle;
            open_read_stream.length = blob.length;
            open_read_stream.fragment_hashes = blob.fragment_hashes.clone();

            let mut ack = OpenNextInboxStreamAck::new();

            ack.type_ = open_next_inbox_stream_ack::Type::Accept.into();

            use ::protobuf::MessageField;

            ack.read_stream = MessageField::some(open_read_stream);

            self.blob_list.lock().unwrap().insert(stream_handle, blob);

            return Ok(ack);
        } else {
            let mut ack = OpenNextInboxStreamAck::new();

            ack.type_ = open_next_inbox_stream_ack::Type::Nomore.into();

            return Ok(ack);
        }
    }

    /// Read fragment
    async fn read_fragment(
        &mut self,
        read_fragment: ReadFragment,
    ) -> anyhow::Result<ReadFragmentAck> {
        let blob = self
            .blob_list
            .lock()
            .unwrap()
            .get(&read_fragment.stream_handle)
            .map(|c| c.clone());

        if blob.is_none() {
            let mut ack = ReadFragmentAck::new();

            ack.ack_type = read_fragment_ack::Type::Break.into();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        let data = self
            .blob
            .read_blob_fragment(&blob.id, read_fragment.offset)
            .await?;

        let mut ack = ReadFragmentAck::new();

        ack.ack_type = read_fragment_ack::Type::Continue.into();

        ack.stream_handle = ack.stream_handle;
        ack.offset = read_fragment.offset;
        ack.content = data;

        Ok(ack)
    }

    /// Close read stream.
    async fn close_inbox_stream(
        &mut self,
        close_inbox_stream: CloseInboxStream,
    ) -> anyhow::Result<CloseInboxStreamAck> {
        let blob = self
            .blob_list
            .lock()
            .unwrap()
            .remove(&close_inbox_stream.stream_handle);

        if blob.is_none() {
            let mut ack = CloseInboxStreamAck::new();

            ack.sync_error = SyncError::Resource.into();

            return Ok(ack);
        }

        let blob = blob.unwrap();

        self.blob.end_write_blob(&blob.id).await?;

        if close_inbox_stream.mark_as_read {
            self.timeline.pop(&blob.mns, &blob.id).await?;
        }

        let mut ack = CloseInboxStreamAck::new();

        ack.stream_handle = close_inbox_stream.stream_handle;

        return Ok(ack);
    }

    /// Read inbox information of [`mns`](MNSAccount)
    async fn open_inbox(&mut self, mns: MNSAccount) -> anyhow::Result<Inbox> {
        self.timeline.status(&mns).await
    }
}
