use crate::{HashedFqn, Ptr, UObject};
use once_cell::sync::Lazy;
use std::{cell::UnsafeCell, collections::HashMap};

pub unsafe trait Cache {
    fn lookup(&self, hfqn: &HashedFqn) -> Ptr<UObject>;
    fn flush(&self);
}

// Pray
pub struct RacyCache(UnsafeCell<HashMap<HashedFqn, Ptr<UObject>>>);
unsafe impl Send for RacyCache {}
unsafe impl Sync for RacyCache {}

unsafe impl Cache for RacyCache {
    fn lookup(&self, hfqn: &HashedFqn) -> Ptr<UObject> {
        unsafe {
            let map = self.0.get();
            let object = (*map)
                .entry(*hfqn)
                .or_insert_with(|| UObject::get_by_fqn(*hfqn).expect("Failed to find UObject"));
            *object
        }
    }

    fn flush(&self) {
        unsafe {
            let map = self.0.get();
            (*map).clear();
        }
    }
}

// TODO: Add mutex cache
// #[cfg(feature = "racy")]
pub const DEFAULT_CACHE: Lazy<RacyCache> = Lazy::new(|| RacyCache(UnsafeCell::default()));
