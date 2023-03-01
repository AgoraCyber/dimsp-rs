use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use rand::{rngs::OsRng, RngCore};

use crate::{
    blob::{Blob, BlobProvider},
    timeline::TimelineProvider,
    DimspStorage, StorageError,
};
use dimsp_types::*;
#[derive(Debug, Default, Clone)]
struct MockTimelineImpl {
    timeline: HashMap<u64, Vec<Blob>>,
    client_read_offsets: HashMap<PublicKey, u64>,
}

#[derive(Debug, Default, Clone)]
pub struct MockTimeline {
    inner: Arc<Mutex<MockTimelineImpl>>,
}

#[async_trait]
impl TimelineProvider for MockTimeline {
    async fn append(&mut self, blob: Blob) -> anyhow::Result<u64> {
        let mut inner = self.inner.lock().unwrap();

        match inner.timeline.get_mut(&blob.mns.uns.id) {
            Some(time_line) => {
                time_line.push(blob);
                return Ok(time_line.len() as u64);
            }
            _ => {}
        }

        inner.timeline.insert(blob.mns.uns.id, vec![blob]);

        Ok(1)
    }

    async fn get(&mut self, account: &MNSAccount) -> anyhow::Result<Option<Blob>> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.client_read_offsets.contains_key(&account.client_id) {
            inner
                .client_read_offsets
                .insert(account.client_id.clone(), 0);
        }

        let offset = inner.client_read_offsets.get(&account.client_id).unwrap();

        match inner.timeline.get(&account.uns.id) {
            Some(time_line) => {
                if (*offset as usize) < time_line.len() {
                    return Ok(time_line.first().map(|b| b.clone()));
                }
            }
            _ => {}
        }

        return Ok(None);
    }

    async fn pop(&mut self, account: &MNSAccount) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.client_read_offsets.contains_key(&account.client_id) {
            return Ok(());
        }
        let offset = inner
            .client_read_offsets
            .get_mut(&account.client_id)
            .unwrap();

        *offset += 1;

        return Ok(());
    }

    async fn status(&mut self, account: &MNSAccount) -> anyhow::Result<Inbox> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.client_read_offsets.contains_key(&account.client_id) {
            inner
                .client_read_offsets
                .insert(account.client_id.clone(), 0);
        }

        let offset = inner.client_read_offsets.get(&account.client_id).unwrap();

        match inner.timeline.get(&account.uns.id) {
            Some(time_line) => {
                if (*offset as usize) < time_line.len() {
                    let mut inbox = Inbox::new();

                    let mut total_length = 0;

                    for index in (*offset as usize)..time_line.len() {
                        total_length = time_line[index].length;
                    }

                    inbox.unread = time_line.len() as u64 - *offset;

                    inbox.total_length = total_length;

                    return Ok(inbox);
                }
            }
            _ => {}
        }

        Ok(Inbox::new())
    }
}
#[derive(Default, Clone)]
struct MockBlobImpl {
    memory: HashMap<[u8; 32], Vec<Vec<u8>>>,
    fragments: HashMap<[u8; 32], Vec<Hash32>>,
    quotas: HashMap<u64, u64>,
}

#[derive(Default, Clone)]
pub struct MockBlob {
    inner: Arc<Mutex<MockBlobImpl>>,
}

#[async_trait]
impl BlobProvider for MockBlob {
    async fn start_write_blob(
        &mut self,
        mns: &MNSAccount,
        length: u64,
        fragment_hashes: Vec<Hash32>,
    ) -> anyhow::Result<Blob> {
        let mut inner = self.inner.lock().unwrap();

        let quota = match inner.quotas.get(&mns.uns.id) {
            Some(quota) => {
                if *quota + length > mns.quota {
                    return Err(StorageError::Quotas((*quota + length - mns.quota) as usize).into());
                } else {
                    *quota + length
                }
            }
            None => length,
        };

        inner.quotas.insert(mns.uns.id, quota);

        let mut id = [0; 32];

        OsRng.fill_bytes(&mut id);

        inner.fragments.insert(id, fragment_hashes.clone());
        inner.memory.insert(id, vec![]);

        Ok(Blob {
            id,
            mns: mns.to_owned(),
            length,
            fragment_hashes,
            next_fragment: 0,
        })
    }

    /// Write blob fragment
    async fn write_blob_fragment(
        &mut self,
        id: &[u8; 32],
        offset: u64,
        data: Vec<u8>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();

        let fragment_hashes = inner
            .fragments
            .get(id)
            .ok_or(StorageError::BlobNotFound(id.to_owned()))?;

        if offset as usize > fragment_hashes.len() {
            return Err(
                StorageError::FragmentOutOfRange(offset as usize, fragment_hashes.len()).into(),
            );
        }

        let expect_hash = fragment_hashes[offset as usize].clone();
        let calc_hash = Hash32::from(keccack256(&data));

        if calc_hash != expect_hash {
            return Err(StorageError::FragmentHash(expect_hash, calc_hash).into());
        }

        match inner.memory.get_mut(id) {
            Some(buff) => {
                if offset as usize != buff.len() {
                    return Err(StorageError::FragmentOffset(buff.len(), offset as usize).into());
                }

                buff.push(data);
            }
            _ => {
                return Err(StorageError::BlobNotFound(id.clone()).into());
            }
        }

        Ok(())
    }

    /// End writing one blob
    async fn end_write_blob(&mut self, _id: &[u8; 32]) -> anyhow::Result<()> {
        Ok(())
    }

    /// Manual invoke `GC` to release blob.
    /// Operation may or may not release blob referenced by id,
    /// this depend on how many id reference to this blob
    async fn remove_blob(&mut self, id: &[u8; 32]) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();

        inner.fragments.remove(id);
        inner.memory.remove(id);

        Ok(())
    }

    async fn start_read_blob(&mut self, id: &[u8; 32]) -> anyhow::Result<()> {
        let inner = self.inner.lock().unwrap();

        _ = inner
            .fragments
            .get(id)
            .ok_or(StorageError::BlobNotFound(id.to_owned()))?;

        Ok(())
    }

    async fn read_blob_fragment(&mut self, id: &[u8; 32], offset: u64) -> anyhow::Result<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        let fragment_hashes = inner
            .fragments
            .get(id)
            .ok_or(StorageError::BlobNotFound(id.to_owned()))?;

        if offset as usize > fragment_hashes.len() {
            return Err(
                StorageError::FragmentOutOfRange(offset as usize, fragment_hashes.len()).into(),
            );
        }

        match inner.memory.get(id) {
            Some(buff) => {
                if offset as usize != buff.len() {
                    return Err(StorageError::FragmentOffset(buff.len(), offset as usize).into());
                }

                Ok(buff[offset as usize].to_owned())
            }
            _ => {
                return Err(StorageError::BlobNotFound(id.clone()).into());
            }
        }
    }
}

/// Define mock storage structure.
pub type MockStorage = DimspStorage<MockTimeline, MockBlob>;

impl Default for MockStorage {
    fn default() -> Self {
        DimspStorage::new(0, Default::default(), Default::default())
    }
}
