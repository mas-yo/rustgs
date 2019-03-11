use std::{
    sync::{Arc,RwLock}
};
use futures::prelude::*;
use tokio::prelude::*;


use crate::{
    get_db,
    command,
    database::*,
    types::*,
    which::*,
};

pub(crate) fn top(peer: Peer) -> impl Future<Item=(Peer,UserID,Option<RoomCode>),Error=()> {

    show_title(peer)
    .and_then(move|peer| {
        peer.send(command::S2C::RequestLoginInfo)
        .map_err(|_|())
    })
    .and_then(move|peer|{
        wait_login_info(peer)
    })
    .and_then(move|(name,peer)|{
        login(&name).map(|(user_id,room_code)|(peer,user_id,room_code))
    })
    //.and_then {
    //  select join_room_queue, if user exists, send peer to room    
    //}
    // .and_then(move|(user_id,room_code)|{
    //     search_room(query_tx2, room_code)
    // })
    // .and_then(move|server_id|{
    // Some => if server_id is self?
    //            if room exists?
    //               send this cl to room
    //            else
    //               create room
    //         else
    //            insert_join_room_queue
    //            send serverid to CL 
    // None => deside server to accept this client, insert_room, insert_join_room_queue
    // })
}

fn show_title(peer: Peer) -> impl Future<Item=Peer,Error=()> {

    peer.send(command::S2C::Message("TITLE".to_string()))
    .and_then(move|peer|{
        peer.send(command::S2C::ShowUI(1))
    })
    .and_then(move|peer|{
        peer.send(command::S2C::ShowUI(2))
    })
    .and_then(move|peer|{
        let (tx,rx) = peer.split();
        // make channel to send tx
        let shared_tx = Arc::new(RwLock::new(Some(tx)));
        let shared_tx2 = shared_tx.clone();
        rx.skip_while(move|cmd|{
            match cmd {
                command::C2S::TouchUI(id) => {
                    if *id == 1 {
                        Ok(false)
                        //send tx via channel
                    }
                    else {
                        let mut locked = shared_tx.write().unwrap();
                        if locked.is_some() {
                            // let tx2 = locked.unwrap();
                            let tx = locked.take().unwrap();
                            let tx = tx.send(command::S2C::Message("hellolo".to_string())).wait().expect("send err");
                            locked.replace(tx);
                            // tokio::spawn(send);
                        }
                        Ok(true)
                    }
                },
                _ => Ok(true)
            }
        })
        .into_future()
        .map_err(|(e,s)|e)
        .and_then(move|(_cmd,rx)|{
            let mut locked = shared_tx2.write().unwrap();
            let tx = locked.take().unwrap();
            let peer = tx.reunite(rx.into_inner()).unwrap();
            Ok(peer)
        })
    })
    .map_err(|_|())
}

fn show_information(peer: Peer) -> impl Future<Item=(),Error=()> {

    peer.send(command::S2C::Message("information: this is sample information".to_string())).map(|_|()).map_err(|_|())
}

fn wait_login_info(peer: Peer) -> impl Future<Item=(String,Peer),Error=()> {

    peer.skip_while(move|cmd|{
        match cmd {
            command::C2S::ResponseLoginInfo(_) => {
                return Ok(false);
            }
            _ => {
                Ok(true)
            }
        }
    })
    .into_future()
    .map_err(|_|())
    .and_then(|(cmd,skipwhile)|{
        if let Some(command::C2S::ResponseLoginInfo(name)) = cmd {
            Ok((name,skipwhile.into_inner()))
        }
        else {
            Err(())
        }
    })
}

fn login(name: &str) -> impl Future<Item=(UserID,Option<RoomCode>),Error=()> {

    let name2 = name.to_string();
    let query = format!("SELECT id,room_code FROM users WHERE name='{}'", name);
    get_db().new_query(&query).map(move|row| {
        let (id,room_code):(u32,Option<u32>) = mysql::from_row(row);
        match room_code {
            None => (UserID::from(id), None),
            Some(code) => (UserID::from(id), Some(RoomCode::from(code)))
        }
    })
    .collect()
    .and_then(move|res|{

// 本当はテーブルロックしないといけないのでWhichは使えない
// INSERT INTO `tags` (`tag`) VALUES ('myvalue1')
//   ON DUPLICATE KEY UPDATE id=LAST_INSERT_ID(id), `tag`='myvalue1';
// SELECT LAST_INSERT_ID();

        let mut room_code = None;
        if res.is_empty() {
            Which::from_future(new_user(&name2))
        }
        else {
            room_code = res[0].1;
            Which::from_value(res[0].0)
        }
        .map(move|id| (id,room_code) )
    })
}

fn new_user(name: &str) -> impl Future<Item=UserID,Error=()> {

    let query = vec![format!("INSERT INTO users SET name='{}'", name), "SELECT LAST_INSERT_ID()".to_string()];
    get_db().new_query_multi(query).map(move|row| {
        let id:u32 = mysql::from_row(row);
        id
    })
    .collect()
    .map(|res|{
        UserID::from(res[0])
    })
}

fn search_room(room_code:u32) -> impl Future<Item=Option<(u32,u32)>, Error=()> {

    //TODO find least load server
    let query = format!("SELECT id,server_id FROM rooms WHERE room_code={}", room_code);
    get_db().new_query(&query).map(move|row| {
        let (room_id,server_id):(u32,u32) = mysql::from_row(row);
        (room_id,server_id)
    })
    .collect()
    .map(|res|{
        if res.is_empty() {
            None
        }
        else {
            Some(res[0])
        }
    })
}

// fn insert_join_room_queue(query_tx: DBQuerySender, room_id:u32, user_id:u32) {

//     let query = format!("INSERT INTO join_room_queue SET room_id={},user_id={}", room_id, user_id);
//     query_tx.new_query(&query)
// }

// fn join_room() -> impl Future<Item=(),Error=()> {
//     //make room if not exists
//     //join
// }

// fn search_room() -> impl Future<Item=(),Error=()> {
//     //login and make session_token
//     //insert/update rooms set server
//     //insert join_room_queue
// }

