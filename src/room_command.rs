use futures::prelude::*;
use tokio::prelude::*;

use crate::{command, types::*};

//#[derive(Debug)]
pub(crate) enum RoomCommand<S, E>
where
    S: Stream<Item = command::C2S, Error = E> + Sink<SinkItem = command::S2C, SinkError = E>,
{
    // Join_(PeerID),
    Join((S,String)),
}
