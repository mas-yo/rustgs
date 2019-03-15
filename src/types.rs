use core::marker::PhantomData;

use futures::future::*;
use std::{collections::HashMap, fmt, hash::*, sync::Arc, sync::RwLock};

use tokio::codec::Framed;
use tokio::net::TcpStream;

use crate::{command, misc::*, peer, room_command::*, sequence_map::*};

pub(crate) struct ID<V, F>
where
    V: Copy,
{
    value: V,
    phantom: PhantomData<F>,
}
impl<V, F> PartialEq for ID<V, F>
where
    V: PartialEq + Copy,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<V, F> Eq for ID<V, F> where V: PartialEq + Copy {}

impl<V, F> From<V> for ID<V, F>
where
    V: Copy,
{
    fn from(v: V) -> Self {
        Self {
            value: v,
            phantom: PhantomData,
        }
    }
}
impl<V, F> Copy for ID<V, F> where V: Copy {}
impl<V, F> Clone for ID<V, F>
where
    V: Copy + Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            phantom: PhantomData,
        }
    }
}
impl<V, F> fmt::Display for ID<V, F>
where
    V: fmt::Display + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<V, F> fmt::Debug for ID<V, F>
where
    V: fmt::Debug + Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}
impl<V, F> Hash for ID<V, F>
where
    V: Hash + Copy,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

pub(crate) struct UserIDPhantom;
pub(crate) struct RoomIDPhantom;
pub(crate) struct RoomCodePhantom;
pub(crate) struct ServerIDPhantom;
pub(crate) type UserID = ID<u32, UserIDPhantom>;
pub(crate) type RoomID = ID<u32, RoomIDPhantom>;
pub(crate) type RoomCode = ID<u32, RoomCodePhantom>;
pub(crate) type ServerID = ID<u32, ServerIDPhantom>;

pub(crate) const fn default_server_id() -> ServerID {
    ServerID {
        value: 0,
        phantom: PhantomData,
    }
}

pub(crate) type TcpPeer = Framed<TcpStream, command::Codec>;

pub(crate) type RoomCommandSender<S, E> = std::sync::mpsc::SyncSender<RoomCommand<S, E>>;
pub(crate) type RoomCommandReceiver<S, E> = std::sync::mpsc::Receiver<RoomCommand<S, E>>;

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
