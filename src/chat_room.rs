use chrono::*;
use std::{
    
};
use futures::prelude::*;

use crate::{
    database::*,
    peer::*,
    command,
    room_command::*,
    types::*,
    tasks::*,
};

pub(crate) struct ChatMessage {
    time: DateTime<Local>,
    name: String,
    message: String,
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
