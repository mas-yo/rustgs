use bytes::buf::BufMut;
use bytes::BytesMut;
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::io;

use std::str::*;

pub(crate) type UIID = u64;
pub(crate) type ClientGUID = String;
pub(crate) type SessionToken = String;

#[derive(Debug, Clone)]
pub enum C2S {
    ResponseLoginInfo(String),
    TouchUI(UIID),
    InputText(String),
    //    EnterRoom,
}

impl FromStr for C2S {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split(',').collect();

        if let Some(cmd) = splitted.get(0) {
            if *cmd == "response_login_info" {
                return Ok(C2S::ResponseLoginInfo(splitted.get(1).unwrap().to_string()));
            }
            if *cmd == "touch_ui" {
                return Ok(C2S::TouchUI(
                    splitted.get(1).unwrap().parse::<UIID>().unwrap(),
                ));
            }
            if *cmd == "input_text" {
                return Ok(C2S::InputText(splitted.get(1).unwrap().to_string()));
            }
        }

        Err(())
    }
}

#[derive(Debug, Clone)]
pub enum S2C {
    RequestLoginInfo,
    Message(String),
    ShowUI(UIID, bool),
    AddText(UIID, String),
}

impl ToString for S2C {
    fn to_string(&self) -> String {
        match self {
            S2C::RequestLoginInfo => "request_login_info".to_string(),
            S2C::Message(msg) => format!("> {}", msg),
            S2C::ShowUI(ui_id, show) => format!("show_ui,{},{}", ui_id, if *show { 1 } else { 0 }),
            S2C::AddText(ui_id, text) => format!("add_text,{},{}", ui_id, text),
        }
    }
}

#[derive(Default)]
pub struct Codec {
    next_index: usize,
}

impl Codec {
    pub fn new() -> Self {
        Self { next_index: 0 }
    }
}

impl Decoder for Codec {
    type Item = C2S;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        // Look for a byte with the value '\n' in buf. Start searching from the search start index.
        if let Some(newline_offset) = buf[self.next_index..].iter().position(|b| *b == b'\n') {
            let newline_index = newline_offset + self.next_index;

            let line = buf.split_to(newline_index + 1);

            // Trim the `\n` from the buffer because it's part of the protocol,
            // not the data.
            let line = &line[..line.len() - 1];

            let line = from_utf8(&line).expect("invalid utf8 data");

            self.next_index = 0;

            //            let splitted : Vec<&str> = line.split(',').collect();

            if let Ok(cmd) = C2S::from_str(line) {
                return Ok(Some(cmd));
            }

            // if let Some(cmd) = splitted.get(0) {
            //     if *cmd == "response_login_info" {
            //         return Ok(Some(C2S::ResponseLoginInfo(splitted.get(1).unwrap().to_string())));
            //         // return Ok(Some(C2S::ResponseLoginInfo(splitted.get(1).unwrap().to_string(), splitted.get(2).unwrap().to_string())));
            //     }
            //     if *cmd == "touch_ui" {
            //         return Ok(Some(C2S::TouchUI(splitted.get(1).unwrap().parse::<UIID>().unwrap())));
            //     }
            //     if *cmd == "input_text" {
            //         return Ok(Some(C2S::InputText(splitted.get(1).unwrap().to_string())));
            //     }
            //     // if *cmd == "enter_room" {
            //     //     return Ok(Some(C2S::EnterRoom));
            //     // }
            // }

            panic!("unknown command");
        } else {
            self.next_index = buf.len();

            Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = S2C;
    type Error = io::Error;

    fn encode(&mut self, cmd: S2C, buf: &mut BytesMut) -> Result<(), io::Error> {
        // It's important to reserve the amount of space needed. The `bytes` API
        // does not grow the buffers implicitly.
        // Reserve the length of the string + 1 for the '\n'.

        let mut line = cmd.to_string();
        // match cmd {
        //     S2C::RequestLoginInfo => {
        //         line = "request_login_info".to_string();
        //     }
        //     S2C::Message(msg) => {
        //         line = format!("> {}", msg);
        //     }
        //     S2C::ShowUI(ui_id) => {
        //         line = format!("show_ui,{}", ui_id);
        //     }
        //     S2C::Result_Login(token) => {
        //         line = "result_login".to_string() + &token;
        //     }
        //     _ => panic!("cant encode"),
        // }

        buf.reserve(line.len() + 1);
        buf.put(line);
        buf.put_u8(b'\n');

        Ok(())
    }
}
