use core::marker::PhantomData;

use std::{
    sync::Arc,
    sync::RwLock,
    collections::HashMap,
    fmt,
};
use futures::future::*;

use tokio::net::{TcpStream};
use tokio::codec::{Framed};

use crate::{
    misc::*,
    command,
    room_command::*,
    sequence_map::*,
    peer,
};

pub(crate) struct ID<V,F> where V:Copy {
    value: V,
    phantom: PhantomData<F>,
}
impl<V,F> PartialEq for ID<V,F> where V:PartialEq+Copy {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<V,F> From<V> for ID<V,F> where V:Copy{
    fn from(v: V) -> Self {
        Self {value:v, phantom: PhantomData}
    }
}
impl<V,F> Copy for ID<V,F> where V:Copy {}
impl<V,F> Clone for ID<V,F> where V:Copy+Clone {
    fn clone(&self) -> Self {
        Self { value: self.value.clone(), phantom: PhantomData}
    }
}
impl<V,F> fmt::Display for ID<V,F> where V:fmt::Display+Copy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

pub(crate) struct UserIDPhantom;
pub(crate) struct RoomCodePhantom;
pub(crate) type UserID = ID<u32,UserIDPhantom>;
pub(crate) type RoomCode = ID<u32,RoomCodePhantom>;

pub(crate) type Peer = Framed<TcpStream,command::Codec>;

pub(crate) type RoomCommandSender = std::sync::mpsc::SyncSender<RoomCommand>;
pub(crate) type RoomCommandReceiver = std::sync::mpsc::Receiver<RoomCommand>;

// pub(crate) type SharedPeers = peer::SharedPeers<Codec>;
// pub(crate) type RoomPeersRx = peer::RoomPeersRx<Codec>;
// pub(crate) type RoomPeersTx = peer::RoomPeersTx<Codec>;

// pub fn new_shared_peers() -> SharedPeers {
//     Arc::new(RwLock::new(SequenceMap::new()))
// }
// pub fn new_room_peers_tx() -> RoomPeersTx {
//     Arc::new(RwLock::new(HashMap::new()))
// }
// pub fn new_room_peers_rx() -> RoomPeersRx {
//     Arc::new(RwLock::new(HashMap::new()))
// }

// pub(crate) trait TokioFuture : Future<Item=(),Error=()> {
// }
// impl<T> TokioFuture for T where T: Future<Item=(),Error=()> {
// }