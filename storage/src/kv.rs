use anyhow::Result;

use async_trait::async_trait;
use dimsp_types::Mime;
use libipld::Cid;

/// Ipld kv database.
#[async_trait]
pub trait MimeKV {
    /// Put mime object into database and generate cid.
    async fn put(&mut self, mime: Mime) -> Result<Cid>;

    /// Returns true if the database contains a mime object for the specified cid.
    async fn contains_cid(&mut self, cid: Cid) -> Result<bool>;

    /// Try get mime object for specified cid. returns [`None`] if object doesn't exist
    async fn get(&mut self, cid: Cid) -> Result<Option<Mime>>;

    /// Delete mime object for the specified cid. returns removed object.
    async fn delete(&mut self, cid: Cid) -> Result<Option<Mime>>;
}
