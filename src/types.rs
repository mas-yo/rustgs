use std::{
    sync::Arc,
    sync::RwLock,
    collections::HashMap,
};
use futures::future::*;
use crate::{
    misc::*,
    command::*,
    room_command::*,
    sequence_map::*,
    peer,
};

pub(crate) type RoomCommandSender = std::sync::mpsc::SyncSender<RoomCommand>;
pub(crate) type RoomCommandReceiver = std::sync::mpsc::Receiver<RoomCommand>;
pub(crate) type SharedPeers = peer::SharedPeers<Codec>;
pub(crate) type RoomPeersRx = peer::RoomPeersRx<Codec>;
pub(crate) type RoomPeersTx = peer::RoomPeersTx<Codec>;

pub fn new_shared_peers() -> SharedPeers {
    Arc::new(RwLock::new(SequenceMap::new()))
}
pub fn new_room_peers_tx() -> RoomPeersTx {
    Arc::new(RwLock::new(HashMap::new()))
}
pub fn new_room_peers_rx() -> RoomPeersRx {
    Arc::new(RwLock::new(HashMap::new()))
}

pub(crate) trait TokioFuture : Future<Item=(),Error=()> {
}
impl<T> TokioFuture for T where T: Future<Item=(),Error=()> {
}