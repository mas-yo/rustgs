use core::marker::PhantomData;

use futures::future::*;
use std::{collections::HashMap, fmt, hash::*, sync::Arc, sync::RwLock};

use tokio::codec::Framed;
use tokio::net::TcpStream;

use crate::{command, misc::*, peer, room_command::*, sequence_map::*};

pub(crate) struct ID<F> {
    value: u64,
    phantom: PhantomData<F>,
}
impl<F> PartialEq for ID<F> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<F> Eq for ID<F> {}

impl<F> From<u64> for ID<F> {
    fn from(v: u64) -> Self {
        Self {
            value: v,
            phantom: PhantomData,
        }
    }
}
impl<F> Copy for ID<F> {}
impl<F> Clone for ID<F> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            phantom: PhantomData,
        }
    }
}
impl<F> fmt::Display for ID<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<F> fmt::Debug for ID<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}
impl<F> Hash for ID<F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<F> ID<F> {
    pub fn new() -> Self {
        Self {
            value: 1,
            phantom: PhantomData,
        }
    }
    pub fn next(&mut self) {
        if self.value == std::u64::MAX {
            panic!("id reached max");
        }
        self.value += 1;
    }
}

pub(crate) struct UserIDPhantom;
pub(crate) struct RoomIDPhantom;
pub(crate) struct RoomCodePhantom;
pub(crate) struct ServerIDPhantom;
pub(crate) struct PeerIDPhantom;
pub(crate) struct CommandSeqIDPhantom;
pub(crate) type UserID = ID<UserIDPhantom>;
pub(crate) type RoomID = ID<RoomIDPhantom>;
pub(crate) type RoomCode = ID<RoomCodePhantom>;
pub(crate) type ServerID = ID<ServerIDPhantom>;
pub(crate) type PeerID = ID<PeerIDPhantom>;
pub(crate) type CommandSeqID = ID<CommandSeqIDPhantom>;

pub(crate) const fn default_server_id() -> ServerID {
    ServerID {
        value: 0,
        phantom: PhantomData,
    }
}

pub(crate) type TcpPeer = Framed<TcpStream, command::Codec>;

pub(crate) type RoomCommandSender<S, E> = std::sync::mpsc::SyncSender<RoomCommand<S, E>>;
pub(crate) type RoomCommandReceiver<S, E> = std::sync::mpsc::Receiver<RoomCommand<S, E>>;
pub(crate) type RoomCommandAsyncSender<S, E> = futures::sync::mpsc::Sender<RoomCommand<S, E>>;
pub(crate) type RoomCommandAsyncReceiver<S, E> = futures::sync::mpsc::Receiver<RoomCommand<S, E>>;

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
