use chrono::*;
use std::{
    collections::HashMap,
    fmt::*,
};
use futures::prelude::*;

use crate::{
    database::*,
    peer::*,
    command,
    room_command::*,
    types::*,
    tasks::*,
    misc::*,
};

pub(crate) struct ChatMessage {
    time: DateTime<Local>,
    name: String,
    message: String,
}

pub(crate) fn chat_room_2<S,E>(room_id: RoomID) -> RoomCommandSender<S,E>
    where S: Stream<Item=command::C2S,Error=E> + Sink<SinkItem=command::S2C,SinkError=E>, E:Display+Debug {
    
    let (tx,rx) = std::sync::mpsc::sync_channel(12);
    tx.clone()
}

pub(crate) fn chat_room<S,E>(room_id: RoomID) -> RoomCommandSender<S,E>
    where S:'static+ Stream<Item=command::C2S,Error=E> + Sink<SinkItem=command::S2C,SinkError=E> + Send, E:'static +Display+Debug {

    // let mut rooms = ROOMS.write().unwrap();

    // if let Some(tx) = rooms.get(&room_id) {
    //     return tx.clone();
    // }

    let (room_tx,room_rx) = std::sync::mpsc::sync_channel::<RoomCommand<S,E>>(12);
//    rooms.insert(room_id, room_tx.clone());

    let mut next_peer_id = 0;
    let mut peer_txs = HashMap::new();
    let mut peer_rxs = HashMap::new();

    let room = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(100))
    .for_each(move|_|{
        match room_rx.try_recv() {
            Ok(RoomCommand::Join(peer)) => {
                println!("room id:{} peer:{} joined", room_id, next_peer_id);
                let (tx,rx) = peer.split();
                peer_txs.insert(next_peer_id, tx);
                peer_rxs.insert(next_peer_id, rx);
                next_peer_id += 1;
            },
            _ => {
            }
        }

        for (_,rx) in peer_rxs.iter_mut() {
            match rx.poll() {
                Ok(Async::Ready(Some(command::C2S::InputText(msg)))) => {
                    for (_,tx) in peer_txs.iter_mut() {
                        tx.send(command::S2C::Message(msg.clone())).wait();
                    }
                },
                _ => {}
            }
        }

        Ok(())
    })
    .map_err(|_|());

    tokio::spawn(room);

    println!("room id {} created", room_id);
    room_tx
//    Ok(()).into_future()
}

// pub(crate) fn chat_room(query_tx: DBQuerySender) -> (impl TokioFuture,RoomCommandSender) {

//     //shiritori
//     //keep message
    
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

//         if let Ok(res) = room_rx.try_recv() {
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
