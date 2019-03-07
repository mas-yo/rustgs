use std::{
    prelude::v1::*,
    sync::Arc,
    sync::RwLock,
};

use tokio::net::{TcpListener,TcpStream};
use tokio::codec::{Framed};
use tokio::prelude::stream::SplitStream;
use lazy_static::lazy_static;

use futures::{
    prelude::*,
    stream::*,
    sync::mpsc,
};

mod misc;
mod room_command;
mod command;
mod sequence_map;
mod peer;
mod database;
mod types;
mod tasks;
mod chat_room;
mod top;

use crate::{
    database::*,
    types::*,
    room_command::*,
    tasks::*,
    misc::*,
    top::*,
    chat_room::*,
};

lazy_static! {
  static ref DB: Arc<RwLock<DBQuerySender>> = {
      let db = start_database();
      tokio::spawn(db.0);
      Arc::new(RwLock::new(db.1))
  };
}

pub(crate) fn get_db() -> DBQuerySender {
    DB.read().unwrap().clone()
}

fn make_server() -> impl Future<Item=(),Error=()> {

    let addr = "127.0.0.1:18290".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let db = Ok(()).into_future().and_then(|_| get_db().new_query("SELECT 1").collect());

    // let (lobby,room_tx) = make_lobby(query_tx.clone());

    let mut next_peer_id = 1;
    let listener = listener.incoming().map_err(|_|()).for_each(move|socket| {
        //ロビーの機能、ここに書いていいんじゃない
        let framed = Framed::new(socket, command::Codec::new());

        top(framed).and_then(|(peer, user_id,room_code)|{
            println!("login ok {},{}", user_id, room_code);

            let chat_room = chat_room();
            tokio::spawn(chat_room);

            Ok(())
            //TODO process user_id,room_code,peer
        })

        // room_tx.send(RoomCommand::Join((next_peer_id,framed)));
        // next_peer_id += 1;
//        Ok(())
    });

    listener.join(db).map(|_|())
}

fn main() { 

    let server = make_server();

    tokio::run(server);
}





// struct SendCommand {
//     peer_id: peer::PeerID,
//     command: command::S2C,
//     peers_tx: RoomPeersTx,
// }
// impl SendCommand {
//     fn new(peer_id: peer::PeerID, command: command::S2C, peers_tx: RoomPeersTx) -> Self {
//         Self {
//             peer_id, command, peers_tx,
//         }
//     }
// }

// impl Future for SendCommand {
//     type Item = ();
//     type Error = ();
//     fn poll(&mut self) -> Poll<(),()> {
//         // what if holding the peer_tx, not holding shared txs taking the peer on each poll
        
//         let mut peers = self.peers_tx.write().expect("lock error");
//         let tx = peers.get_mut(&self.peer_id);
//         if tx.is_none() {
//             return Err(()); //エラーでいいかな
//         }
//         match tx.unwrap().start_send(self.command.clone()) {
//             Ok(_) => {
//                 Ok(Async::Ready(()))
//             },
//             Err(_) => {
//                 Err(())
//             }
//         }
//     }
// }

// struct FlushCommand {
//     peer_id: peer::PeerID,
//     peers_tx: RoomPeersTx,
// }
// impl FlushCommand {
//     fn new(peer_id: peer::PeerID, peers_tx: RoomPeersTx) -> Self {
//         Self {
//             peer_id, peers_tx,
//         }
//     }
// }
// impl Future for FlushCommand {
//     type Item = ();
//     type Error = ();
//     fn poll(&mut self) -> Poll<(),()> {
//         let mut peers = self.peers_tx.write().expect("lock error");
//         let tx = peers.get_mut(&self.peer_id);
//         if tx.is_none() {
//             return Err(()); //エラーでいいかな
//         }
//         tx.unwrap().poll_complete().map_err(|_|())
//     }
// }
// struct ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
//     peer_id: peer::PeerID,
//     peers_rx: RoomPeersRx,
//     handler: F,
// }
// impl<F> ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
//     fn new(peer_id: peer::PeerID, peers_rx: RoomPeersRx, handler:F) -> Self {
//         Self { peer_id, peers_rx, handler }
//     }
// }
// impl<F> Future for ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
//     type Item = ();
//     type Error = ();
//     fn poll(&mut self) -> Poll<(),()> {
//         let mut peers = self.peers_rx.write().expect("lock error");
//         let rx = peers.get_mut(&self.peer_id);
//         if rx.is_none() {
//             return Err(());
//         }

//         match rx.unwrap().poll() {
//             Ok(Async::Ready(Some(cmd))) => {
//                 match (self.handler)(cmd) {
//                     Ok(_) => Ok(Async::Ready(())),
//                     Err(_) => Err(()),
//                 }
//             },
//             Ok(Async::Ready(None)) => {
//                 Err(()) //?
//             },
//             Ok(Async::NotReady) => {
//                 Ok(Async::NotReady)
//             },
//             Err(_) => {
//                 Err(())
//             }
//         }
        
//     }
// }

// struct SendQuery {
//     query_str: String,
//     query_tx: DBQuerySender,
// }
// impl SendQuery {
//     fn new(query_str: String, query_tx: DBQuerySender) -> Self {
//         Self { query_str, query_tx }
//     }
// }
// impl Future for SendQuery {
//     type Item = DBResultReceiver;
//     type Error = ();
//     fn poll(&mut self) -> Poll<DBResultReceiver,()> {
//         Ok(Async::Ready(self.query_tx.new_query(&self.query_str)))
//     }
// }

// type CommandStream = SplitStream<Framed<TcpStream,command::C2S>>;
// type CommandSink = SplitSink<Framed<TcpStream,command::S2C>>;

// fn receive_once<S,P>(rx: S, mut pred: P) -> impl Future<Item=S,Error=S>
//  where S:Stream, P:FnMut(&S::Item)->bool {
//     rx.take_while(move|cmd|{
//         if pred(cmd) {
//             Ok(true)
//         } else {
//             println!("unexpected cmd");
//             Ok(false)
//         }
//     })
//     .take(1)
//     .into_future()
//     .map_err(|(_e,stream)|{
//         println!("recv error");
//         stream.into_inner().into_inner()
//     })
//     .map(|(_,stream)|stream.into_inner().into_inner())
// }

// fn send<S>(tx: S, item: S::SinkItem) -> impl Future<Item=S,Error=()> where S:Sink {
//     tx.send(item)
//     .map_err(|e|{
//         println!("send err");
//         ()
//     })
// }

// fn join_room(peer: Framed<TcpStream,command::Codec>, query_tx: DBQuerySender, session_token: command::SessionToken) -> impl TokioFuture {

//     let query = format!("SELECT rooms.id, rooms.code FROM room_queue INNER JOIN rooms ON rooms.id == room_queue.room_id WHERE session_token='{}'", session_token);
//     query_tx.new_query(&query).map(move|row|{
//         let (room_id,room_code):(u64,u32) = mysql::from_row(row);
//         (room_id,room_code)
//     })
//     .collect()
//     .and_then(|results|{
//         match results.get(0) {
//             None => {
//                 Err(())
//             },
//             Some((room_id,room_code)) => {
//                 Err(())
//             }
//         }
//     })
// }
