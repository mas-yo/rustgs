use std::{
    env,
    fmt::Display,
    net::{SocketAddr, ToSocketAddrs},
    prelude::v1::*,
    str::FromStr,
    sync::Arc,
    sync::RwLock,
};
use websocket::{codec::ws::MessageCodec, message::OwnedMessage, r#async::Server, result::*};

use lazy_static::lazy_static;
use tokio::codec::Framed;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::stream::SplitStream;

use futures::{prelude::*, stream::*, sync::mpsc};

mod chat_room;
mod command;
mod database;
mod listener;
mod misc;
mod peer;
mod room_command;
mod sequence_map;
mod tasks;
mod top;
mod types;
mod which;
//mod room;

use crate::{
    chat_room::*, database::*, listener::*, misc::*, room_command::*, tasks::*, top::*, types::*,
    which::*,
};

static mut SERVER_ID: ServerID = default_server_id();

pub(crate) fn get_server_id() -> ServerID {
    unsafe { SERVER_ID }
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

lazy_static! {
    pub(crate) static ref ROOMS_WS: ArcHashMap<RoomID, RoomCommandSender<WsPeer, WebSocketError>> =
        { new_arc_hash_map() };
}
lazy_static! {
    pub(crate) static ref ROOMS_WS_ASYNC: ArcHashMap<RoomID, RoomCommandAsyncSender<WsPeer, WebSocketError>> =
        { new_arc_hash_map() };
}
lazy_static! {
    pub(crate) static ref ROOMS_TCP: ArcHashMap<RoomID, RoomCommandSender<TcpPeer, std::io::Error>> =
        { new_arc_hash_map() };
}
lazy_static! {
    pub(crate) static ref ROOMS_TCP_ASYNC: ArcHashMap<RoomID, RoomCommandAsyncSender<TcpPeer, std::io::Error>> =
        { new_arc_hash_map() };
}

fn new_room(room_code: RoomCode) -> impl Future<Item = RoomID, Error = ()> {
    let query = vec![
        format!("INSERT INTO rooms SET code={},server_id={}", room_code, 1),
        "SELECT LAST_INSERT_ID()".to_string(),
    ];
    get_db()
        .new_query_multi(query)
        .map(move |row| {
            let room_id: u64 = mysql::from_row(row);
            RoomID::from(room_id)
        })
        .collect()
        .and_then(|res| Ok(res[0]))
}

fn find_room(room_code: RoomCode) -> impl Future<Item = (RoomID, ServerID), Error = ()> {
    Ok(()).into_future().and_then(move |_| {
        let db = sync_db();
        let mut db_lock = db.write().unwrap();
        db_lock.query("LOCK TABLES rooms WRITE").unwrap();
        let existing: Vec<(u64, u64, u32)> = db_lock
            .query(format!(
                "SELECT id,server_id,player_count FROM rooms WHERE code={} ORDER BY player_count ASC LIMIT 1",
                room_code
            ))
            .unwrap()
            .map(move |row| {
                let row = row.unwrap();
                mysql::from_row::<(u64, u64, u32)>(row)
            })
            .collect();

        let mut new_room = false;
        if existing.is_empty() { new_room = true; }
        else if existing[0].2 >= 2 { new_room = true; }

        let room_id: RoomID;
        let server_id: ServerID;
        if new_room {
            db_lock
                .query(format!(
                    "INSERT INTO rooms SET code={},player_count=1,server_id={}",
                    room_code,
                    get_server_id()
                ))
                .unwrap();

            let new_id: Vec<u64> = db_lock
                .query("SELECT LAST_INSERT_ID()".to_string())
                .unwrap()
                .map(move |row| {
                    let row = row.unwrap();
                    let room_id: u64 = mysql::from_row(row);
                    room_id
                })
                .collect();

            room_id = RoomID::from(new_id[0]);
            server_id = get_server_id();
        } else {
            room_id = RoomID::from(existing[0].0);
            server_id = ServerID::from(existing[0].1);
            db_lock.query(format!("UPDATE rooms SET player_count=player_count+1 WHERE id={}", room_id));
        }
        db_lock.query("UNLOCK TABLES").unwrap();

        Ok((room_id, server_id))
    })
}

fn new_server<T>(addr: &T) -> impl Future<Item = ServerID, Error = ()>
where
    T: Display,
{
    let query = vec![
        format!("INSERT INTO servers SET address='{}'", addr),
        "SELECT LAST_INSERT_ID()".to_string(),
    ];
    get_db()
        .new_query_multi(query)
        .map(move |row| {
            let id: u64 = mysql::from_row(row);
            ServerID::from(id)
        })
        .collect()
        .and_then(|res| Ok(res[0]))
}

fn find_server_id(addr: SocketAddr) -> impl Future<Item = ServerID, Error = ()> {
    Ok(()).into_future().and_then(move |_| {
        let db = sync_db();
        let mut db_lock = db.write().unwrap();
        db_lock.query("LOCK TABLES servers WRITE").unwrap();
        let existing: Vec<u64> = db_lock
            .query(format!("SELECT id FROM servers WHERE address='{}'", addr))
            .unwrap()
            .map(move |row| {
                let row = row.unwrap();
                mysql::from_row::<u64>(row)
            })
            .collect();

        let server_id: ServerID;
        if existing.is_empty() {
            db_lock
                .query(format!("INSERT INTO servers SET address='{}'", addr))
                .unwrap();

            let new_id: Vec<u64> = db_lock
                .query("SELECT LAST_INSERT_ID()".to_string())
                .unwrap()
                .map(move |row| {
                    let row = row.unwrap();
                    mysql::from_row::<u64>(row)
                })
                .collect();

            server_id = ServerID::from(new_id[0]);
        } else {
            server_id = ServerID::from(existing[0]);
        }
        db_lock.query("UNLOCK TABLES").unwrap();

        Ok(server_id)
    })
}

fn make_tcp_server(addr: SocketAddr, sync_mode: String) -> impl Future<Item = (), Error = ()> {
    let server = Ok(())
        .into_future()
        .and_then(|_| get_db().new_query("SELECT 1").collect())
        .and_then(move |_| find_server_id(addr))
        .and_then(move |server_id| {
            unsafe {
                SERVER_ID = ServerID::from(server_id);
            }
            make_tcpsocket_listener::<command::Codec>(&addr).for_each(move |peer| {
                let sync_mode = sync_mode.clone();
                // make_websocket_listener(&addr).for_each(|peer| {
                let top = top(peer)
                    .and_then(move |(peer, user_id, name, opt_room_code)| {
                        let mut room_code = RoomCode::from(1);
                        match opt_room_code {
                            Some(code) => {
                                println!("login ok {},{}", user_id, code);
                                room_code = code;
                            }
                            None => {
                                println!("login ok {}", user_id);
                            }
                        }
                        find_room(room_code).and_then(move |(room_id, server_id)| {
                            println!("room id:{} server_id:{}", room_id, server_id);

                            let join_command = room_command::RoomCommand::Join((peer, name));
                            if sync_mode == "sync" {
                                let mut rooms = ROOMS_TCP.write().unwrap();

                                let room_tx;
                                if let Some(tx) = rooms.get(&room_id) {
                                    room_tx = tx.clone();
                                } else {
                                    room_tx = chat_room::<TcpPeer, std::io::Error>(room_id);
                                    rooms.insert(room_id, room_tx.clone());
                                }
                                room_tx.send(join_command);
                                Ok(())
                            } else {
                                let mut rooms = ROOMS_TCP_ASYNC.write().unwrap();

                                let room_tx;
                                if let Some(tx) = rooms.get(&room_id) {
                                    room_tx = tx.clone();
                                } else {
                                    room_tx = chat_room_async::<TcpPeer, std::io::Error>(room_id);
                                    rooms.insert(room_id, room_tx.clone());
                                }
                                room_tx.send(join_command).wait();
                                Ok(())
                            }
                        })
                    })
                    .map(|_| ());
                tokio::spawn(top);
                Ok(())
            })
        });
    server
}

fn make_websocket_server(
    addr: SocketAddr,
    sync_mode: String,
) -> impl Future<Item = (), Error = ()> {
    let server = Ok(())
        .into_future()
        .and_then(|_| get_db().new_query("SELECT 1").collect())
        .and_then(move |_| find_server_id(addr))
        .and_then(move |server_id| {
            unsafe {
                SERVER_ID = ServerID::from(server_id);
            }
            make_websocket_listener(&addr).for_each(move |peer| {
                let sync_mode = sync_mode.clone();
                // make_websocket_listener(&addr).for_each(|peer| {
                let top = top(peer)
                    .and_then(move |(peer, user_id, name, opt_room_code)| {
                        let mut room_code = RoomCode::from(1);
                        match opt_room_code {
                            Some(code) => {
                                println!("login ok {},{}", user_id, code);
                                room_code = code;
                            }
                            None => {
                                println!("login ok {}", user_id);
                            }
                        }
                        find_room(room_code).and_then(move |(room_id, server_id)| {
                                println!("room id:{} server_id:{}", room_id, server_id);

                                let join_command = room_command::RoomCommand::Join((peer, name));
                                if sync_mode == "sync" {
                                    let mut rooms = ROOMS_WS.write().unwrap();

                                    let room_tx;
                                    if let Some(tx) = rooms.get(&room_id) {
                                        room_tx = tx.clone();
                                    } else {
                                        room_tx = chat_room::<
                                            WsPeer,
                                            websocket::result::WebSocketError,
                                        >(room_id);
                                        rooms.insert(room_id, room_tx.clone());
                                    }
                                    room_tx.send(join_command);
                                    Ok(())
                                } else {
                                    let mut rooms = ROOMS_WS_ASYNC.write().unwrap();

                                    let room_tx;
                                    if let Some(tx) = rooms.get(&room_id) {
                                        room_tx = tx.clone();
                                    } else {
                                        room_tx = chat_room_async::<
                                            WsPeer,
                                            websocket::result::WebSocketError,
                                        >(room_id);
                                        rooms.insert(room_id, room_tx.clone());
                                    }
                                    room_tx.send(join_command).wait();
                                    Ok(())
                                }
                            })
                    })
                    .map(|_| ());
                tokio::spawn(top);
                Ok(())
            })
        });
    server
}

#[cfg(feature = "protocol_ws")]
fn make_server(addr: SocketAddr, sync_mode: String) -> impl Future<Item = (), Error = ()> {
    println!("websocket server mode:{}", sync_mode);
    make_websocket_server(addr, sync_mode)
}
#[cfg(feature = "protocol_tcp")]
fn make_server(addr: SocketAddr, sync_mode: String) -> impl Future<Item = (), Error = ()> {
    println!("tcp server mode:{}", sync_mode);
    make_tcp_server(addr, sync_mode)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("usage: {} ipaddr [sync|async]", args[0]);
        return;
    }

    let addr = SocketAddr::from_str(&format!("{}:18290", args[1])).unwrap();
    let sync_mode = args[2].clone();

    let server = make_server(addr, sync_mode);
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
