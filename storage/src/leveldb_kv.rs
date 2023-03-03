use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use async_trait::async_trait;
use dimsp_types::{keccack256, Mime};

use libipld::{
    cbor::DagCborCodec,
    multihash::{Code, MultihashDigest},
    prelude::Codec,
    Cid,
};

use crate::kv::MimeKV;

pub struct LeveldbMimeKV {
    db: Arc<Mutex<rusty_leveldb::DB>>,
}

impl LeveldbMimeKV {
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

#[async_trait]
impl MimeKV for LeveldbMimeKV {
    async fn contains_cid(&mut self, cid: Cid) -> Result<bool> {
        let mut db = self.db.lock().unwrap();

        let key = keccack256(&cid.to_bytes());

        Ok(db.get(&key).is_some())
    }

    async fn delete(&mut self, cid: libipld::Cid) -> Result<Option<Mime>> {
        let mut db = self.db.lock().unwrap();

        let key = keccack256(&cid.to_bytes());

        db.delete(&key)?;

        let key = cid.to_bytes();

        let mime = db.get(&key);

        db.delete(&key)?;

        if let Some(mime) = mime {
            return Ok(DagCborCodec.decode(&mime)?);
        } else {
            return Ok(None);
        }
    }

    async fn get(&mut self, cid: libipld::Cid) -> Result<Option<Mime>> {
        let mut db = self.db.lock().unwrap();

        let mime = db.get(&cid.to_bytes());

        if let Some(mime) = mime {
            return Ok(DagCborCodec.decode(&mime)?);
        } else {
            return Ok(None);
        }
    }

    async fn put(&mut self, mime: dimsp_types::Mime) -> Result<Cid> {
        let mut db = self.db.lock().unwrap();

        let data = DagCborCodec.encode(&mime)?;

        let cid = Cid::new_v1(DagCborCodec.into(), Code::Keccak256.digest(&data));

        let cid_bytes = cid.to_bytes();

        let key = keccack256(&cid_bytes);

        db.put(&key, &[0u8; 1])?;

        db.put(&cid_bytes, &data)?;

        Ok(cid)
    }
}

#[cfg(test)]
mod tests {
    use std::{env, time::SystemTime};

    use dimsp_types::Mime;
    use hex::ToHex;
    use libipld::{
        cbor::DagCborCodec,
        multihash::{Code, MultihashDigest},
        Cid,
    };
    use rand::{rngs::OsRng, RngCore};

    use crate::kv::MimeKV;

    use super::LeveldbMimeKV;

    #[async_std::test]
    async fn test_kv() {
        _ = pretty_env_logger::try_init();

        let mut buff = [0u8; 32];
        OsRng.fill_bytes(&mut buff);
        let mut kv =
            LeveldbMimeKV::local(env::temp_dir().join(buff.encode_hex::<String>())).unwrap();

        let mime = Mime {
            id: Cid::new_v1(DagCborCodec.into(), Code::Blake2b256.digest(&b""[..])),
            length: 10,
            content: [0u8; 1024].to_vec(),
            multipart: vec![
                Cid::new_v1(DagCborCodec.into(), Code::Blake2b256.digest(&b""[..])),
                Cid::new_v1(DagCborCodec.into(), Code::Blake2b256.digest(&b""[..])),
            ],
        };

        let now = SystemTime::now();

        for _ in 0..100 {
            kv.put(mime.clone()).await.unwrap();
        }

        log::debug!("elapsed {:?}", now.elapsed().unwrap() / 100);
    }
}
