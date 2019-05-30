use std::{fmt::*, net::SocketAddr, prelude::v1::*, str::*};
use websocket::{codec::ws::MessageCodec, message::OwnedMessage, r#async::Server, result::*};

use tokio::codec::{Decoder, Encoder, Framed};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::stream::SplitStream;

use futures::prelude::*;

use crate::command;

pub(crate) struct WsPeer {
    framed: Framed<TcpStream, MessageCodec<OwnedMessage>>,
}

// impl WsPeer {
//     fn new(framed: Framed<TcpStream,MessageCodec<OwnedMessage>>) -> Self {
//         Self {framed:framed}
//     }
// }

impl Stream for WsPeer {
    type Item = command::C2S;
    type Error = WebSocketError;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.framed.poll() {
            Ok(Async::Ready(Some(msg))) => {
                match msg {
                    OwnedMessage::Text(txt) => {
                        Ok(Async::Ready(Some(command::C2S::from_str(&txt).unwrap())))
                    }
                    OwnedMessage::Close(_) => Ok(Async::Ready(None)),
                    OwnedMessage::Ping(data) => {
                        // self.framed.start_send(OwnedMessage::Pong(data)); TODO
                        Ok(Async::NotReady)
                    }
                    _ => Err(WebSocketError::ProtocolError("invalid data type")),
                }
                // if let OwnedMessage::Text(txt) = msg {
                //     return Ok(Async::Ready(Some(command::C2S::from_str(&txt).unwrap())));
                // } else {
                //     Err(WebSocketError::ProtocolError("invalid data type"))
                // }
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

impl Sink for WsPeer {
    type SinkItem = command::S2C;
    type SinkError = WebSocketError;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let s = item.to_string();
        let cloned_item = item.clone();
        self.framed
            .start_send(OwnedMessage::Text(s))
            .map(move |asyncsink| match asyncsink {
                AsyncSink::Ready => AsyncSink::Ready,
                AsyncSink::NotReady(_) => AsyncSink::NotReady(cloned_item),
            })
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed.poll_complete()
        // Ok(Async::Ready(()))
    }
}

pub(crate) fn make_websocket_listener(addr: &SocketAddr) -> impl Stream<Item = WsPeer, Error = ()> {
    let handle = tokio::reactor::Handle::current();
    let server = Server::bind(addr, &handle).unwrap();

    server
        .incoming()
        .map_err(|_| println!("ws incoming error"))
        .map(move |(upgrade, _)| {
            upgrade
                .accept()
                .wait()
                .map(|(socket, _)| WsPeer { framed: socket })
                .unwrap() //TODO
        })
}

pub(crate) fn make_tcpsocket_listener<Codec>(
    addr: &SocketAddr,
) -> impl Stream<Item = Framed<TcpStream, Codec>, Error = ()>
where
    Codec: Decoder<Item = command::C2S, Error = std::io::Error>
        + Encoder<Item = command::S2C, Error = std::io::Error>
        + Default,
{
    let listener = TcpListener::bind(addr).unwrap();

    listener
        .incoming()
        .and_then(|socket| {
            socket.set_nodelay(true);
            Ok(socket)
        })
        .map_err(|_| println!("tcp incoming error"))
        .map(move |socket| Framed::new(socket, Codec::default()))
}
