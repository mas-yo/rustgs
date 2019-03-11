use std::{
    prelude::v1::*,
    sync::Arc,
    sync::RwLock,
    net::{SocketAddr,ToSocketAddrs},
    fmt::Display,
    str::FromStr,
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
mod which;

use crate::{
    database::*,
    types::*,
    room_command::*,
    tasks::*,
    misc::*,
    top::*,
    chat_room::*,
    which::*,
};

static mut SERVER_ID: ServerID = new_server_id();

pub(crate) fn get_server_id() -> ServerID {
    unsafe {
        SERVER_ID
    }
}

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

fn new_room(room_code: RoomCode) -> impl Future<Item=RoomID,Error=()> {
    let query = vec![format!("INSERT INTO rooms SET code={},server_id={}", room_code, 1),"SELECT LAST_INSERT_ID()".to_string()];
    get_db().new_query_multi(query)
    .map(move|row|{
        let room_id:u32 = mysql::from_row(row);
        RoomID::from(room_id)
    })
    .collect()
    .and_then(|res|{
        Ok(res[0])
    })
}

fn find_room(room_code: RoomCode) -> impl Future<Item=RoomID,Error=()> {

    get_db().new_query_multi(vec!["LOCK TABLES rooms WRITE".to_string(), format!("SELECT id,player_count,server_id FROM rooms WHERE code={}", room_code)])
    .map(move|row| {
        let (room_id,count,server_id):(u32,u32,u32) = mysql::from_row(row);
        (RoomID::from(room_id),count,ServerID::from(server_id))
    })
    .collect()
    .and_then(move|res|{
        if res.is_empty() {
            Which::from_future(new_room(room_code))
        }
        else {
            let min_count = res.iter().fold((RoomID::from(0),0u32,ServerID::from(0)),|a,b| (b.0,std::cmp::min(a.1,b.1),b.2) );
            Which::from_value(min_count.0)
        }
    })
    .then(move|res|{
        get_db().new_query("UNLOCK TABLES").collect()
        .and_then(move|_|res)
    })

    // Ok(RoomID::from(1)).into_future()
}

fn new_server<T>(addr: &T) -> impl Future<Item=ServerID,Error=()> where T: Display {
    let query = vec![format!("INSERT INTO servers SET address='{}'", addr),"SELECT LAST_INSERT_ID()".to_string()];
    get_db().new_query_multi(query)
    .map(move|row|{
        let id:u32 = mysql::from_row(row);
        ServerID::from(id)
    })
    .collect()
    .and_then(|res|{
        Ok(res[0])
    })
}

fn find_server_id<T>(addr: &T) -> impl Future<Item=ServerID,Error=()> where T: Display+Clone {

    let query = format!("SELECT id FROM servers WHERE address='{}'", addr);
    let addr2 = addr.clone();
    get_db().new_query_multi(vec!["LOCK TABLES servers WRITE".to_string(), query])
    .map(move|row|{
        let id:u32 = mysql::from_row(row);
        ServerID::from(id)
    })
    .collect()
    .and_then(move|res|{
        if res.is_empty() {
            Which::from_future(new_server(&addr2))
        }
        else {
            Which::from_value(res[0])
        }
    })
    .then(move|res|{
        get_db().new_query("UNLOCK TABLES").collect()
        .and_then(move|_|res)
    })

    // Ok(ServerID::from(0)).into_future()
}

fn make_server(addr: SocketAddr) -> impl Future<Item=(),Error=()> {

    find_server_id(&addr)
    .and_then(move|server_id|{
        unsafe{
            SERVER_ID = ServerID::from(server_id);
        }
        let listener = TcpListener::bind(&addr).unwrap();

        println!("server id: {}", get_server_id());

        // let mut next_peer_id = 1;
        listener.incoming().map_err(|_|()).for_each(move|socket| {
            let framed = Framed::new(socket, command::Codec::new());

            top(framed).and_then(|(peer, user_id,opt_room_code)|{

                let mut room_code = RoomCode::from(1);
                match opt_room_code {
                    Some(code) => { 
                        println!("login ok {},{}", user_id, code); 
                        room_code = code;
                    },
                    None => { println!("login ok {}", user_id); }
                }

                find_room(room_code)
                .and_then(|room_id|{
                    println!("room id {}", room_id);
                    Ok(())
                })
            })
            .map(|_|())
        })
        .map(|_|())
    })
}

fn main() { 

    let server = Ok(()).into_future().and_then(|_|{
        get_db().new_query("SELECT 1").collect()
    })
    .and_then(|_|{
        make_server(SocketAddr::from_str("127.0.0.1:18290").unwrap())
    });
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
