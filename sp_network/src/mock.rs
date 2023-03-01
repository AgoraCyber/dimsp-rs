use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use dimsp_types::*;

use crate::SpNetwork;

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
    async fn subscription(&self, mns_id: u64) -> anyhow::Result<Option<Vec<SPRSAccount>>> {
        let inner = self.inner.lock().unwrap();

        Ok(inner.subscribed_by.get(&mns_id).map(|c| c.clone()))
    }
}
