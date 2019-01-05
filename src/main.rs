use std::prelude::v1::*;
use tokio::net::{TcpListener,TcpStream};
use tokio::codec::{Framed};
use tokio::prelude::stream::SplitStream;

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
};


struct SendCommand {
    peer_id: peer::PeerID,
    command: command::S2C,
    peers_tx: RoomPeersTx,
}
impl SendCommand {
    fn new(peer_id: peer::PeerID, command: command::S2C, peers_tx: RoomPeersTx) -> Self {
        Self {
            peer_id, command, peers_tx,
        }
    }
}

impl Future for SendCommand {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(),()> {
        // what if holding the peer_tx, not holding shared txs taking the peer on each poll
        
        let mut peers = self.peers_tx.write().expect("lock error");
        let tx = peers.get_mut(&self.peer_id);
        if tx.is_none() {
            return Err(()); //エラーでいいかな
        }
        match tx.unwrap().start_send(self.command.clone()) {
            Ok(_) => {
                Ok(Async::Ready(()))
            },
            Err(_) => {
                Err(())
            }
        }
    }
}

struct FlushCommand {
    peer_id: peer::PeerID,
    peers_tx: RoomPeersTx,
}
impl FlushCommand {
    fn new(peer_id: peer::PeerID, peers_tx: RoomPeersTx) -> Self {
        Self {
            peer_id, peers_tx,
        }
    }
}
impl Future for FlushCommand {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(),()> {
        let mut peers = self.peers_tx.write().expect("lock error");
        let tx = peers.get_mut(&self.peer_id);
        if tx.is_none() {
            return Err(()); //エラーでいいかな
        }
        tx.unwrap().poll_complete().map_err(|_|())
    }
}
struct ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
    peer_id: peer::PeerID,
    peers_rx: RoomPeersRx,
    handler: F,
}
impl<F> ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
    fn new(peer_id: peer::PeerID, peers_rx: RoomPeersRx, handler:F) -> Self {
        Self { peer_id, peers_rx, handler }
    }
}
impl<F> Future for ReceiveCommand<F> where F:FnMut(command::C2S) -> Result<(),()> {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(),()> {
        let mut peers = self.peers_rx.write().expect("lock error");
        let rx = peers.get_mut(&self.peer_id);
        if rx.is_none() {
            return Err(());
        }

        match rx.unwrap().poll() {
            Ok(Async::Ready(Some(cmd))) => {
                match (self.handler)(cmd) {
                    Ok(_) => Ok(Async::Ready(())),
                    Err(_) => Err(()),
                }
            },
            Ok(Async::Ready(None)) => {
                Err(()) //?
            },
            Ok(Async::NotReady) => {
                Ok(Async::NotReady)
            },
            Err(_) => {
                Err(())
            }
        }
        
    }
}

struct SendQuery {
    query_str: String,
    query_tx: DBQuerySender,
}
impl SendQuery {
    fn new(query_str: String, query_tx: DBQuerySender) -> Self {
        Self { query_str, query_tx }
    }
}
impl Future for SendQuery {
    type Item = DBResultReceiver;
    type Error = ();
    fn poll(&mut self) -> Poll<DBResultReceiver,()> {
        Ok(Async::Ready(self.query_tx.new_query(&self.query_str)))
    }
}

type CommandStream = SplitStream<Framed<TcpStream,command::C2S>>;
type CommandSink = SplitSink<Framed<TcpStream,command::S2C>>;

fn receive_once<S,P>(rx: S, mut pred: P) -> impl Future<Item=S,Error=S>
 where S:Stream, P:FnMut(&S::Item)->bool {
    rx.take_while(move|cmd|{
        if pred(cmd) {
            Ok(true)
        } else {
            println!("unexpected cmd");
            Ok(false)
        }
    })
    .take(1)
    .into_future()
    .map_err(|(_e,stream)|{
        println!("recv error");
        stream.into_inner().into_inner()
    })
    .map(|(_,stream)|stream.into_inner().into_inner())
}

fn send<S>(tx: S, item: S::SinkItem) -> impl Future<Item=S,Error=()> where S:Sink {
    tx.send(item)
    .map_err(|e|{
        println!("send err");
        ()
    })
}

fn join_room(peer: Framed<TcpStream,command::Codec>, query_tx: DBQuerySender, session_token: command::SessionToken) -> impl TokioFuture {

    let query = format!("SELECT rooms.id, rooms.code FROM room_queue INNER JOIN rooms ON rooms.id == room_queue.room_id WHERE session_token='{}'", session_token);
    query_tx.new_query(&query).map(move|row|{
        let (room_id,room_code):(u64,u32) = mysql::from_row(row);
        (room_id,room_code)
    })
    .collect()
    .and_then(|results|{
        match results.get(0) {
            None => {
                Err(())
            },
            Some((room_id,room_code)) => {
                Err(())
            }
        }
    })
}

// fn make_lobby(query_tx: DBQuerySender) -> (impl TokioFuture,RoomCommandSender) {

//     let (room_tx,room_rx) = std::sync::mpsc::sync_channel(32);
//     let peers_rx = new_room_peers_rx();
//     let peers_tx = new_room_peers_tx();
//     let next_tasks = new_shared_tasks();
//     let mut tasks = new_tasks();

//     let update = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(100))
//     .and_then(move|_| {

//         {
//             let mut v = next_tasks.write().unwrap();
//             tasks.append(&mut v);
//         }
//         tasks.update();

//         // println!("room timer {:?}", thread::current().name());
//         if let Ok(res) = room_rx.try_recv() {
//             // println!("room cmd: {:?}", res);
//             let peers_tx = peers_tx.clone();
//             let peers_rx = peers_rx.clone();
//             // let query_tx = query_tx.clone();
//             // let query_tx2 = query_tx.clone();
//             match res {
//                 RoomCommand::Join((peer_id,peer)) => {
//                     //tx,rx一緒のままでいいのかも
//                     //query_txもresultに含めるとよい
//                     // let (mut tx,rx) = framed.split();
//                     {
//                         let test = send(peer, command::S2C::Message("TITLE".to_string()))
//                         .and_then(move|peer|{
//                             send(peer, command::S2C::ShowUI(1))
//                         })
//                         .and_then(move|peer|{
//                             receive_once(peer, |cmd|{
//                                 match cmd {
//                                     command::C2S::TouchUI(id) => {
//                                         if *id == 1 { true } else { false }
//                                     },
//                                     _ => false
//                                 }
//                             })
//                             .map_err(|_| ())
//                         })
//                         .and_then(move|peer| {
//                             send(peer, command::S2C::RequestLoginInfo)
//                         })
//                         .and_then(move|peer| {
//                             let recv_cl_id;
//                             let recv_token;
//                             receive_once(peer, |cmd|{
//                                 match cmd {
//                                     command::C2S::ResponseLoginInfo(cl_id, token) => {
//                                         recv_cl_id = cl_id.clone();
//                                         recv_token = token.clone();
//                                         true
//                                     },
//                                     _ => false
//                                 }
//                             })
//                             .map(|_| (peer, query_tx, recv_cl_id, recv_token))
//                             .map_err(|_|())
//                         })
//                         // .and_then(move|(peer, cl_guid, session_token)| {
//                             // if !session_token.is_empty() {
//                             //     let query = format!("SELECT room_id FROM room_queue WHERE session_token='{}'", session_token);
//                             //     query_tx.new_query(&query).map(move|row|{
//                             //         let room_id = mysql::from_row(row);
//                             //         room_id
//                             //     })
//                             //     .collect()
//                             //     .map(|ids| ((peer, ids.get(0))))
//                             // }
//                             // else {
//                             //     let query = format!("SELECT name FROM accounts WHERE cl_guid='{}'", cl_guid);
//                             //     query_tx.new_query(&query).map(move|row| {
//                             //         let name:String = mysql::from_row(row);
//                             //         name
//                             //     })
//                             //     .collect()
//                             //     .map(|names| (peer,query_tx,names[0].clone()) )
//                             // }
//                         // })
//                         .and_then(move|(peer,query_tx,cl_id,token)| {
//                             send(peer, command::S2C::Message(format!("hello, {}", cl_id))).map(|peer| (peer,query_tx))
//                         })
//                         // .and_then(move|(peer,query_tx)|{
//                         //     //TODO lock table
//                         //     let search_room = query_tx.new_query("SELECT id,server_id,player_count FROM rooms WHERE code=1 AND player_count < 10 ORDER BY player_count").map(move|row|{
//                         //         let (room_id,server_id,player_count) = mysql::from_row(row);
//                         //         //room token??
//                         //         server_id
//                         //     })
//                         //     .collect();

//                         //     let search_server = query_tx.new_query("SELECT SUM(player_count) AS player_count,server_id FROM rooms GROUP BY server_id ORDER BY player_count LIMIT 1").map(move|row|{
//                         //         let (_, server_id) = mysql::from_row(row);
//                         //         server_id
//                         //     })
//                         //     .collect();

//                         //     //make room if server_id matches

//                         //     // search_room.join(search_server)
//                         //     // .map(move|(find_room,find_server)|{
//                         //     //     (peer,query_tx,find_room,find_server)
//                         //     // })
//                         // })
//                         // .and_then(move|(peer,query_tx,find_room,find_server)|{

//                         //     //room token->Room instanceのhashmapが要る
//                         //     if !find_room.is_empty() {
//                         //         //send(peer, reconnect)
//                         //     }
//                         //     if !find_server.is_empty() {
//                         //     //todo insert room

//                         //     }

//                         // })
//                         .map_err(|_|())
//                         .map(|_|());
//                         // .map(move|(tx,rx)|{
//                         //     peers_tx.write().unwrap().insert(1, tx);
//                         //     peers_rx.write().unwrap().insert(1, rx);
//                         //     ()
//                         // });
//                         tokio::spawn(test);
//                         // next_tasks.write().unwrap().push_task(test);
//                     }
//                     // {
//                     //     let show_title = SendCommand::new(peer_id, command::S2C::Message("TITLE".to_string()), peers_tx.clone());
//                     //     let show_ui = SendCommand::new(peer_id, command::S2C::ShowUI(1), peers_tx.clone());
//                     //     let flush = FlushCommand::new(peer_id, peers_tx.clone());
//                     //     let receive = ReceiveCommand::new(peer_id, peers_rx.clone(), |cmd|{
//                     //         match cmd {
//                     //             command::C2S::TouchUI(ui_id) => {
//                     //                 if ui_id != 1 {
//                     //                     return Err(());
//                     //                 }
//                     //                 println!("Touch!!!!");
//                     //                 Ok(())
//                     //             }
//                     //         }
//                     //     });

//                     //     let send_query = SendQuery::new("SELECT name FROM accounts WHERE id=1".to_string(), query_tx.clone());

//                     //     // let (result_tx,result_rx) = mpsc::channel(1);
//                     //     // tokio::spawn(query_tx.clone().send(("SELECT name FROM accounts WHERE id=1".to_string(), result_tx)).map(|_|()).map_err(|_|())); //TODO future result
//                     //     let peers_tx2 = peers_tx.clone();
//                     //     let peers_tx3 = peers_tx.clone();

//                     //     let query_task = send_query.and_then(|rx|{
//                     //         rx.map(move|row|{
//                     //             let name:String = mysql::from_row(row);
//                     //             name
//                     //         })
//                     //         .collect()
//                     //     })
//                     //     .and_then(move|names|{
//                     //         SendCommand::new(peer_id, command::S2C::Result_Login(names[0].clone()), peers_tx2)
//                     //     })
//                     //     .and_then(move|_|{
//                     //         FlushCommand::new(peer_id, peers_tx3)
//                     //     });

//                     //     let task = show_title.and_then(move|_|show_ui).and_then(move|_|flush).and_then(move|_|receive).and_then(move|_|query_task);
//                     //     next_tasks.write().unwrap().push_task(task);
//                     //     // tokio::spawn(s.join(f).map(|_|()));
//                     // }
//                     // peers_tx.lock_insert(peer_id, tx);
//                     // peers_rx.lock_insert(peer_id, rx);
//                 }
//             }
//         }

//         Ok(())
//     }).for_each(move|_|{
// //        println!("and then?");
//         Ok(())
//     }).map_err(|_| {
//         println!("room err");
//     });

//     (update,room_tx)
// }

fn make_server() -> impl Future<Item=(),Error=()> {

    let addr = "127.0.0.1:18290".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let (db_task,query_tx) = start_database();

    // let (lobby,room_tx) = make_lobby(query_tx.clone());

    let mut next_peer_id = 1;
    let listener = listener.incoming().map_err(|_|()).for_each(move|socket| {
        //ロビーの機能、ここに書いていいんじゃない
        let framed = Framed::new(socket, command::Codec::new());

        top(framed, query_tx.clone()).map(|_|())
        //TODO process user_id,room_code,peer

        // room_tx.send(RoomCommand::Join((next_peer_id,framed)));
        // next_peer_id += 1;
//        Ok(())
    });

    // listener.select(lobby).map(|(_,sn)|{
    //     tokio::spawn(sn);
    // }).map_err(|_|()).select(db_task).map(|(_,sn)|{
    //     tokio::spawn(sn);
    // }).map_err(|_| ())
    listener.join(db_task).map(|_|())
}

fn main() { 

    let server = make_server();

    tokio::run(server);
}