use chrono::*;
use futures::prelude::*;
use std::{collections::*, fmt::*, sync::*};

use crate::{command, database::*, get_db, misc::*, peer::*, room_command::*, tasks::*, types::*};

pub(crate) struct ChatMessage {
    name: String,
    message: String,
}

type CommandQueue = VecDeque<(CommandSeqID, Option<PeerID>, command::S2C)>;

struct SendQueue<S, E>
where
    S: Sink<SinkItem = command::S2C, SinkError = E>,
{
    peer_tx: Arc<RwLock<S>>,
    commands: VecDeque<command::S2C>,
}

impl<S, E> Future for SendQueue<S, E>
where
    S: Sink<SinkItem = command::S2C, SinkError = E>,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut tx = self.peer_tx.write().unwrap();
        loop {
            match self.commands.front() {
                None => {
                    return Ok(Async::Ready(()));
                }
                Some(cmd) => match tx.start_send(cmd.clone()) {
                    Ok(AsyncSink::Ready) => {
                        self.commands.pop_front();
                    }
                    Ok(AsyncSink::NotReady(_)) => {
                        return Ok(Async::NotReady);
                    }
                    Err(_) => {
                        return Err(());
                    }
                },
            }
            //TODO poll_complete;
        }
    }
}

enum AsyncSendItem<P, D> {
    Peer(P),
    SendData(D),
}

fn async_sender<S, I>() -> futures::sync::mpsc::Sender<(Option<PeerID>, AsyncSendItem<S, I>)>
where
    S: 'static + Send + Sink<SinkItem = I>,
    I: 'static + Send + Clone + Debug,
    S::SinkError: Debug,
{
    let mut peers_tx = HashMap::new();
    let (tx, rx) = futures::sync::mpsc::channel::<(Option<PeerID>, AsyncSendItem<S, I>)>(32);
    let task = rx.for_each(move |(peer_id, item)| {
        match item {
            AsyncSendItem::Peer(peer) => {
                if peer_id.is_some() {
                    peers_tx.insert(peer_id.unwrap(), peer.wait());
                }
            }
            AsyncSendItem::SendData(data) => {
                peers_tx.retain(|id, tx| {
                    if peer_id.is_some() && peer_id.unwrap() != *id {
                        return true;
                    }
                    // println!("send {:?} to {}", data, id);
                    if let Err(e) = tx.send(data.clone()) {
                        // println!("send err! {:?}", e);
                        return false;
                    }
                    tx.flush();
                    true
                });
            }
        }
        Ok(())
    });

    tokio::spawn(task.map_err(|_| ()));

    tx
}

pub(crate) fn chat_room_async<S, E>(room_id: RoomID) -> RoomCommandAsyncSender<S, E>
where
    S: 'static
        + Stream<Item = command::C2S, Error = E>
        + Sink<SinkItem = command::S2C, SinkError = E>
        + Send,
    E: 'static + Display + Debug + Send,
{
    let (room_tx, room_rx) = futures::sync::mpsc::channel::<RoomCommand<S, E>>(12);
    let messages = Arc::new(RwLock::new(VecDeque::<ChatMessage>::new()));

    let sender = async_sender();
    let mut sender1 = sender.clone().wait();

    let mut next_peer_id = PeerID::new();
    let recv_room_command = room_rx.for_each(move |cmd| {
        match cmd {
            RoomCommand::Join((peer, name)) => {
                let (tx, rx) = peer.split();

                if let Err(e) = sender1.send((Some(next_peer_id), AsyncSendItem::Peer(tx))) {
                    println!("send join error, {}", e);
                }

                {
                    if let Err(e) = sender1.send((
                        Some(next_peer_id),
                        AsyncSendItem::SendData(command::S2C::ShowUI(2, true)),
                    )) {
                        println!("show ui 2 send error {}", e);
                    }
                    let messages = messages.read().unwrap();
                    for msg in messages.iter() {
                        let txt = format!("{}: {}", msg.name, msg.message);
                        if let Err(e) = sender1.send((
                            Some(next_peer_id),
                            AsyncSendItem::SendData(command::S2C::AddText(2001, txt)),
                        )) {
                            println!("text send error {}", e);
                        }
                    }

                    next_peer_id.next();
                }
                let messages = messages.clone();
                let mut sender2 = sender.clone().wait();
                let recv_msg = rx.for_each(move |cmd| {
                    // println!("{:?}", cmd);
                    match cmd {
                        command::C2S::InputText(txt) => {
                            let mut lock = messages.write().unwrap();
                            lock.push_back(ChatMessage {
                                name: name.clone(),
                                message: txt.clone(),
                            });
                            if lock.len() > 100 {
                                lock.pop_front();
                            }

                            let chat_txt = format!("{}: {}", name, txt);
                            if let Err(e) = sender2.send((
                                None,
                                AsyncSendItem::SendData(command::S2C::AddText(
                                    2001,
                                    chat_txt.clone(),
                                )),
                            )) {
                                println!("send text error {}", e);
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                }).and_then(move|_|{
                    get_db().new_query(format!("UPDATE rooms SET player_count=player_count-1 WHERE id={}", room_id));
                    Ok(())
                });
                tokio::spawn(recv_msg.map_err(|_| ()));
            }
        }
        Ok(())
    });

    tokio::spawn(recv_room_command);

    println!("async room id {} created", room_id);
    room_tx
}

pub(crate) fn chat_room<S, E>(room_id: RoomID) -> RoomCommandSender<S, E>
where
    S: 'static
        + Stream<Item = command::C2S, Error = E>
        + Sink<SinkItem = command::S2C, SinkError = E>
        + Send,
    E: 'static + Display + Debug,
{
    let mut messages = VecDeque::<ChatMessage>::new();

    let (room_tx, room_rx) = std::sync::mpsc::sync_channel::<RoomCommand<S, E>>(12);
    //    rooms.insert(room_id, room_tx.clone());

    let mut next_peer_id = 0;
    let mut peer_txs = HashMap::new();
    let mut peer_rxs = HashMap::new();
    let mut room_started = false;

    let room = tokio::timer::Interval::new(
        std::time::Instant::now(),
        std::time::Duration::from_millis(1),
    )
    .skip_while(move |_| {
        match room_rx.try_recv() {
            Ok(RoomCommand::Join((peer, name))) => {
                println!("room id:{} peer:{} joined", room_id, next_peer_id);
                let (tx, rx) = peer.split();
                let tx = tx.send(command::S2C::ShowUI(2, true)).wait().unwrap();

                let mut opt_tx = Some(tx);
                for msg in messages.iter() {
                    let text = format!("{}: {}", msg.name, msg.message);
                    let tx = opt_tx.take().unwrap();
                    opt_tx = Some(tx.send(command::S2C::AddText(2001, text)).wait().unwrap());
                }
                let tx = opt_tx.take().unwrap();

                peer_txs.insert(next_peer_id, tx);
                peer_rxs.insert(next_peer_id, (rx, name));
                next_peer_id += 1;
                room_started = true;
            }
            _ => {}
        }

        peer_rxs.retain(|_, (rx, name)| {
            match rx.poll() {
                Ok(Async::Ready(Some(command::C2S::InputText(msg)))) => {
                    messages.push_back(ChatMessage {
                        name: name.clone(),
                        message: msg.clone(),
                    });
                    if messages.len() > 100 {
                        messages.pop_front();
                    }

                    for (_, tx) in peer_txs.iter_mut() {
                        let text = format!("{}: {}", name, msg);
                        // println!("sending {}", text.len());
                        if tx.send(command::S2C::AddText(2001, text)).wait().is_err() {
                            println!("send error");
                        }
                    }
                }
                Ok(Async::Ready(None)) => {
                    get_db().new_query(format!("UPDATE rooms SET player_count=player_count-1 WHERE id={}", room_id));
                    return false;
                }
                _ => {}
            }
            true
        });

        if room_started && peer_rxs.len() == 0 {
            println!("room id {} closed", room_id);
            Ok(false)
        } else {
            Ok(true)
        }
    })
    .for_each(|_| Ok(()))
    .map_err(|_| ());

    tokio::spawn(room);

    println!("sync room id {} created", room_id);
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
