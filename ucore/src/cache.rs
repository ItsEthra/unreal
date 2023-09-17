use crate::{HashedFqn, Ptr, UObject};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Trait for cache system
pub trait Cache<Key> {
    type Data;

    fn lookup(&self, key: &Key) -> Self::Data;
    fn insert(&self, key: Key, value: Self::Data);

    fn flush(&self);
}

#[cfg(feature = "parking_lot")]
use parking_lot::Mutex;
#[cfg(feature = "spin")]
use spin::Mutex;

type UObjectCache = AnyCache<HashedFqn, Ptr<UObject>>;

pub struct AnyCache<K, V>(Mutex<HashMap<K, V>>);

impl<K, V> Default for AnyCache<K, V> {
    #[inline]
    fn default() -> Self {
        Self(Mutex::default())
    }
}

impl Cache<HashedFqn> for UObjectCache {
    type Data = Ptr<UObject>;

    fn lookup(&self, hfqn: &HashedFqn) -> Ptr<UObject> {
        let mut lock = self.0.lock();
        let object = lock
            .entry(*hfqn)
            .or_insert_with(|| UObject::get_by_fqn(*hfqn).expect("Failed to find UObject"));
        *object
    }

    fn flush(&self) {
        self.0.lock().clear();
    }

    fn insert(&self, key: HashedFqn, value: Ptr<UObject>) {
        self.0.lock().insert(key, value);
    }
}

pub(crate) static DEFAULT_CACHE: Lazy<UObjectCache> = Lazy::new(UObjectCache::default);
