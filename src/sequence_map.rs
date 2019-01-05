use std::{
    prelude::v1::*,
    collections::HashMap,
};

pub type SequenceID = u64;

pub struct SequenceMap<V> {
    map: HashMap<SequenceID,V>,
    seq: SequenceID,
}
impl<V> SequenceMap<V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            seq: 0,
        }
    }
    pub fn new_entry(&mut self, v: V) -> SequenceID {
        self.seq += 1;
        self.map.insert(self.seq, v);
        self.seq
    }
    pub fn get(&self, id: &SequenceID) -> Option<&V> {
        self.map.get(id)
    }
    pub fn get_mut(&mut self, id: &SequenceID) -> Option<&mut V> {
        self.map.get_mut(id)
    }
    pub fn remove(&mut self, id: &SequenceID) -> Option<V> {
        self.map.remove(id)
    }
}
