use std::collections::HashMap;
use std::prelude::v1::*;
use std::sync::{Arc, RwLock};
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use futures::stream::{SplitSink, SplitStream};

use crate::{misc::*, sequence_map::*};

// pub(crate) type PeerID = SequenceID;
// pub(crate) type FramedStream<C> = Framed<TcpStream,C>;

// pub(crate) type SharedPeers<C> = Arc<RwLock<SequenceMap<FramedStream<C>>>>;

// pub(crate) struct SharedPeers<C: Decoder+Encoder> {
//     peers: Arc<RwLock<SequenceMap<FramedStream<C>>>>,
// }
// impl<C: Decoder+Encoder> SharedPeers<C> {
//     pub fn new() -> Self {
//         Self {
//             peers: Arc::new(RwLock::new(SequenceMap::new())),
//         }
//     }
//     pub fn new_peer(&self, f: FramedStream<C>) -> PeerID {
//         self.peers.write_expect().new_entry(f)
//     }
//     pub fn remove(&self, conn_id: &PeerID) -> Option<FramedStream<C>> {
//         self.peers.write_expect().remove(conn_id)
//     }
// }
// impl<C: Decoder+Encoder> Clone for SharedPeers<C> {
//     fn clone(&self) -> Self {
//         Self { peers: self.peers.clone() }
//     }
// }

// pub type RoomPeersTx<C> = ArcHashMap<PeerID,SplitSink<FramedStream<C>>>;
// pub type RoomPeersRx<C> = ArcHashMap<PeerID,SplitStream<FramedStream<C>>>;
