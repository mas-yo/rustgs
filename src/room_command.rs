use tokio::net::{TcpStream};
use tokio::codec::{Framed};

use crate::{
    types::*,
    command,
};

//#[derive(Debug)]
pub(crate) enum RoomCommand {
    // Join_(PeerID),
    Join(Peer),
}
