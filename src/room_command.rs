use tokio::net::{TcpStream};
use tokio::codec::{Framed};

use crate::{
    peer::*,
    command,
};

//#[derive(Debug)]
pub(crate) enum RoomCommand {
    // Join_(PeerID),
    Join((PeerID,Framed<TcpStream,command::Codec>)),
}
