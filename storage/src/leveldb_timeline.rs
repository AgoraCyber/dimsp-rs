use anyhow::Result;
use async_trait::async_trait;
use dimsp_types::MNSAccount;
use libipld::Cid;
use rusty_leveldb::DB;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::Range,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::timeline::Timeline;

pub struct LeveldbTimeline {
    db: Arc<Mutex<rusty_leveldb::DB>>,
}

impl LeveldbTimeline {
    /// Create kv in memory
    pub fn memory() -> Result<Self> {
        let db = rusty_leveldb::DB::open("::memory::", rusty_leveldb::in_memory())?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Create kv database in local storage
    pub fn local<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let db = rusty_leveldb::DB::open(path.into(), Default::default())?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Account {
    start: u64,
    end: u64,
    clients: BTreeMap<String, u64>,
}

impl Account {
    fn first_n(&self, mns: &MNSAccount, n: u64) -> Range<u64> {
        let client_offset = self
            .clients
            .get(&mns.client_id.to_string())
            .map(|c| *c)
            .unwrap_or(0);

        let start = if self.start > client_offset {
            self.start
        } else {
            client_offset
        };

        let end = if start + n > self.end {
            self.end
        } else {
            start + n
        };

        start..end
    }

    fn advance(&mut self, mns: &MNSAccount, steps: u64) -> u64 {
        let key = mns.client_id.to_string();
        let client_offset = self.clients.get(&key).map(|c| *c).unwrap_or(0);

        let to = client_offset + steps;

        let to = if self.start > to { self.start } else { to };

        let to = if self.end < to { self.end } else { to };

        self.clients.insert(key, to);

        self.end - to
    }

    fn length_of(&mut self, mns: &MNSAccount) -> u64 {
        let client_offset = self
            .clients
            .get(&mns.client_id.to_string())
            .map(|c| *c)
            .unwrap_or(0);

        let start = if self.start > client_offset {
            self.start
        } else {
            client_offset
        };

        self.end - start
    }
}

fn get_account(db: &mut MutexGuard<DB>, mns: &MNSAccount) -> Account {
    if let Some(buff) = db.get(&mns.uns.id.to_be_bytes()) {
        serde_json::from_str(&String::from_utf8_lossy(&buff)).unwrap()
    } else {
        Default::default()
    }
}

fn save_account(db: &mut MutexGuard<DB>, mns: &MNSAccount, account: &Account) -> Result<()> {
    let json_str = serde_json::to_string(&account)?;

    log::debug!("{}", json_str);

    db.put(&mns.uns.id.to_be_bytes(), json_str.as_bytes())?;

    Ok(())
}

fn save_cid(db: &mut MutexGuard<DB>, mns: &MNSAccount, offset: u64, cid: Cid) -> Result<()> {
    let key = format!("{}_{}", mns.uns.id, offset);
    db.put(key.as_bytes(), &cid.to_bytes())?;

    Ok(())
}

fn get_cid(db: &mut MutexGuard<DB>, mns: &MNSAccount, offset: u64) -> Result<Cid> {
    let key = format!("{}_{}", mns.uns.id, offset);
    let buff = db.get(key.as_bytes()).ok_or(anyhow::format_err!(
        "Inner constraint: miss mns({}) offset({})",
        mns.uns.id,
        offset
    ))?;

    Ok(Cid::try_from(buff)?)
}

#[async_trait]
impl Timeline for LeveldbTimeline {
    /// Append cid into account's timeline column.
    async fn append(&mut self, mns: MNSAccount, cid: Cid) -> Result<()> {
        let mut db = self.db.lock().unwrap();

        let mut account = get_account(&mut db, &mns);

        let offset = account.end;

        account.end += 1;

        save_cid(&mut db, &mns, offset, cid)?;

        save_account(&mut db, &mns, &account)?;

        Ok(())
    }

    /// Get account's first n cids.
    async fn get(&mut self, mns: MNSAccount, first_n: u64) -> Result<Vec<Cid>> {
        let mut db = self.db.lock().unwrap();

        let account = get_account(&mut db, &mns);

        let mut cids = vec![];

        for i in account.first_n(&mns, first_n as u64) {
            cids.push(get_cid(&mut db, &mns, i)?);
        }

        Ok(cids)
    }

    /// Move timeline cursor to next `n` cid.
    /// if out of range, the cursor will be set to the end of timeline.
    async fn advance(&mut self, mns: MNSAccount, steps: u64) -> Result<u64> {
        let mut db = self.db.lock().unwrap();

        let mut account = get_account(&mut db, &mns);

        let length = account.advance(&mns, steps);

        save_account(&mut db, &mns, &account)?;

        Ok(length)
    }

    async fn length(&mut self, mns: MNSAccount) -> Result<u64> {
        let mut db = self.db.lock().unwrap();

        let mut account = get_account(&mut db, &mns);

        Ok(account.length_of(&mns))
    }
}

#[cfg(test)]
mod tests {
    use dimsp_types::MNSAccount;
    use libipld::{
        cbor::DagCborCodec,
        multihash::{Code, MultihashDigest},
        Cid,
    };

    use crate::timeline::Timeline;

    use super::LeveldbTimeline;

    #[async_std::test]
    async fn test_timeline() {
        let mut timeline = LeveldbTimeline::memory().unwrap();

        let mns = MNSAccount::default();

        let cid1 = Cid::new_v1(DagCborCodec.into(), Code::Blake2b256.digest(&b"1"[..]));

        timeline.append(mns.clone(), cid1).await.unwrap();

        let cid2 = Cid::new_v1(DagCborCodec.into(), Code::Blake2b256.digest(&b"2"[..]));

        timeline.append(mns.clone(), cid2).await.unwrap();

        assert_eq!(timeline.length(mns.clone()).await.unwrap(), 2);

        let cids = timeline.get(mns.clone(), 4).await.unwrap();

        assert_eq!(cids, vec![cid1, cid2]);

        timeline.advance(mns.clone(), 1).await.unwrap();

        let cids = timeline.get(mns.clone(), 4).await.unwrap();

        assert_eq!(cids, vec![cid2]);

        timeline.advance(mns.clone(), 10).await.unwrap();

        let cids = timeline.get(mns.clone(), 4).await.unwrap();

        assert_eq!(cids, vec![]);
    }
}
