extern crate tokio;
extern crate bytes;
extern crate diesel;
extern crate futures;

use std::prelude::v1::*;
use std::thread;
use std::collections::*;
use std::sync::mpsc;
use std::str;
use std::sync::{Arc, Mutex};

use tokio::prelude::*;
use tokio::io;
use tokio::net::TcpListener;
use tokio::codec::{Decoder,Encoder,Framed};
//use futures::sync::mpsc;
//use tokio::tokio_threadpool::blocking::*;
use bytes::BytesMut;
use bytes::buf::BufMut;
use diesel::prelude::*;
use diesel::MysqlConnection;
use diesel::connection::SimpleConnection;

#[derive(Debug)]
enum Command {
    Login,
    EnterRoom,
}


struct LinesCodec {
    next_index: usize,
}

impl LinesCodec {
    fn new() -> Self {
        Self {
            next_index: 0
        }
    }
}

impl Decoder for LinesCodec {
    type Item = Command;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        // Look for a byte with the value '\n' in buf. Start searching from the search start index.
        if let Some(newline_offset) = buf[self.next_index..].iter().position(|b| *b == b'\n')
        {
            let newline_index = newline_offset + self.next_index;

            let line = buf.split_to(newline_index + 1);

            // Trim the `\n` from the buffer because it's part of the protocol,
            // not the data.
            let line = &line[..line.len() - 1];

            let line = str::from_utf8(&line).expect("invalid utf8 data");

            self.next_index = 0;

            let splitted : Vec<&str> = line.split(',').collect();

            if let Some(cmd) = splitted.get(0) {
                if *cmd == "login" {
                    return Ok(Some(Command::Login));
                }
                if *cmd == "enter_room" {
                    return Ok(Some(Command::EnterRoom));
                }
            }

            Ok(Some(Command::Login)) //本当はエラー
        } else {
            self.next_index = buf.len();

            Ok(None)
        }
    }
}

impl Encoder for LinesCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, line: String, buf: &mut BytesMut) -> Result<(), io::Error> {
        // It's important to reserve the amount of space needed. The `bytes` API
        // does not grow the buffers implicitly.
        // Reserve the length of the string + 1 for the '\n'.
        buf.reserve(line.len() + 1);

        buf.put(line);

        buf.put_u8(b'\n');

        Ok(())
    }
}

struct OneByte {
    socket: tokio::net::TcpStream,

}

impl Stream for OneByte {
    type Item = u8;
    type Error = tokio::io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>,Self::Error> {
        let mut buf = [0u8;1];
        let n = futures::try_ready!(self.socket.poll_read(&mut buf));

        if n == 0 {
            println!("discon");
            return Ok(Async::Ready(Option::None));
        }

        Ok(Async::Ready(Some(buf[0])))
    }
}

fn db_thread_main() {
    
}

//コネクションの管理をどうするか？
//Room毎にchannel を作る
//Sharedでroomとchannel:txを対応づける
//Framed::poll内でsharedから対応するtxをみつけて送信
//Room::pollでrxからデータを受け取る

type RoomID = u32;
type ConnID = u32;

type RoomMap = HashMap<RoomID,mpsc::Sender<Command>>;
type ConnMap = HashMap<ConnID,RoomID>;

struct Room {
    rx: mpsc::Receiver<Command>,
}

impl Future for Room {
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<Self::Item,Self::Error> {
        println!("room polled");
        Ok(Async::Ready(()))
    }
}

fn main() { 
    let addr = "127.0.0.1:18290".parse().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let mut conn_id: u32 = 0;
    let rooms = Arc::new(Mutex::new(RoomMap::new()));  //RwLockの方がいい？
    let conns = Arc::new(Mutex::new(ConnMap::new()));

    let (tx,rx) = mpsc::channel();
    {
        let mut locked = rooms.lock().unwrap();
        locked.insert(1, tx);
    }

    let lobby = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(1000))
    .for_each(move|_| {
        println!("fire; {:?}", thread::current().name());
        if let Ok(res) = rx.try_recv() {
            println!("cmd: {:?}", res);
            match res {
                Command::EnterRoom => {
                    let new_room = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(1000))
                    .for_each(move|_| {
                        println!("new {:?}", thread::current().name());
                        Ok(())
                    }).map_err(|_|());
                    tokio::spawn(new_room);
                }
                _ => {}
            }
        }
        Ok(())
    })
    .map_err(|e| {
        println!("interval errored; err={:?}", e);
        ()
    });

//    let (tx,rx) = mpsc::unbounded();

    let server = listener.incoming().for_each(move|socket| {
        conn_id += 1;

        let mut locked_conns = conns.lock().unwrap();
        locked_conns.insert(conn_id, 1); //lobby=1
        let framed_sock = Framed::new(socket, LinesCodec::new());
//        let cloned = tx.clone();
        let cloned_rooms = rooms.clone();
        let cloned_conns = conns.clone();
        let onebyte = framed_sock.for_each(move|result|{
            let locked_conns = cloned_conns.lock().unwrap();
            if let Some(room_id) = locked_conns.get(&conn_id) {
                let locked = cloned_rooms.lock().unwrap();
                if let Some(tx) = locked.get(&room_id) {
                    tx.send(result);
                }
            }

            Ok(())
        }).map_err(|_|());

        tokio::spawn(onebyte);
        Ok(())
    })
    .map_err(|err| {
        println!("accept error = {:?}", err);
        ()
    });

    // MySQL サンプル
    // let mysql = MysqlConnection::establish("mysql://rustgs:@localhost/rustgs").expect("err");
    // mysql.batch_execute("INSERT INTO users(id,name) VALUES(1,'test')").expect("exec err");

    // lobby
    // let timer_task = tokio::timer::Interval::new(std::time::Instant::now(), std::time::Duration::from_millis(1000))
    //     .for_each(move|instant| {
    //         println!("fire; {:?}", thread::current().name());
    //         Ok(())
    //     })
    //    .map_err(|e| {
    //         println!("interval errored; err={:?}", e);
    //         ()
    //    });

    let mut s = server.select(lobby).map(|(_,sn)|{
        tokio::spawn(sn);
    }).map_err(|_| ()); 

    tokio::run(s);
}