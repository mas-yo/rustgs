use std::{
    collections::*,
    sync::*,
};

use crate::{
    sequence_map::*,
};

pub type ArcHashMap<K,V> = Arc<RwLock<HashMap<K,V>>>;
pub type ArcSequenceMap<V> = Arc<RwLock<SequenceMap<V>>>;

pub(crate) fn new_arc_hash_map<K,V>() -> ArcHashMap<K,V> where K:std::cmp::Eq+std::hash::Hash {
    Arc::new(RwLock::new(HashMap::new()))
}

pub(crate) trait WriteExpect<T> {
    fn write_expect(&self) -> RwLockWriteGuard<T>;
}
impl<T> WriteExpect<T> for RwLock<T> {
    fn write_expect(&self) -> RwLockWriteGuard<T> {
        self.write().expect("lock is poisoned!")
    }
}

pub(crate) trait LockInsert<K,V> {
    fn lock_insert(&self, key: K, value: V) -> Option<V>;
}
impl<K,V> LockInsert<K,V> for RwLock<HashMap<K,V>> where K:std::cmp::Eq+std::hash::Hash {
    fn lock_insert(&self, key: K, value: V) -> Option<V> {
        self.write_expect().insert(key, value)
    }
}

pub(crate) trait LockForEach<K,V> {
    fn lock_for_each<F>(&self, f: F) where F:FnMut((& K,&mut V));
}
impl<K,V> LockForEach<K,V> for RwLock<HashMap<K,V>> where K:std::cmp::Eq+std::hash::Hash {
    fn lock_for_each<F>(&self, mut f: F) where F:FnMut((& K,&mut V)) {
        for elm in self.write_expect().iter_mut() {
            f(elm);
        }
    }
}

pub(crate) trait LockNewEntry<V> {
    fn lock_new_entry(&self, v: V) -> SequenceID;
}
impl<V> LockNewEntry<V> for RwLock<SequenceMap<V>> {
    fn lock_new_entry(&self, v: V) -> SequenceID {
        self.write_expect().new_entry(v)
    }
}

// pub(crate) trait LockGet<V> {
//     fn get(&self, id: &SequenceID) -> Option<&V>;
// }
// impl<V> LockGet<V> for RwLock<SequenceMap<V>> {
//     fn get(&self, id: &SequenceID) -> Option<&V> {
//         self.write_expect().get(id)
//     }
// }
pub(crate) trait LockRemove<V> {
    fn lock_remove(&self, id: &SequenceID) -> Option<V>;
}
impl<V> LockRemove<V> for RwLock<SequenceMap<V>> {
    fn lock_remove(&self, id: &SequenceID) -> Option<V> {
        self.write_expect().remove(id)
    }
}
