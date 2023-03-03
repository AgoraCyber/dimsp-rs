use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

mod mns;
pub use mns::*;

mod spss;
pub use spss::*;

mod uns;
pub use uns::*;

mod pubkey;
pub use pubkey::*;

mod hash;
pub use hash::*;

mod sync;
pub use sync::*;

#[derive(Default)]
pub struct IdGenerator(Arc<AtomicU64>);

impl Into<u64> for &mut IdGenerator {
    fn into(self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}
