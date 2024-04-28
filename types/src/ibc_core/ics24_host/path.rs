use derive_more::Display;

use crate::ibc_core::ics04_channel::packet::Sequence;

use super::identifier::{ChannelId, ClientId, ConnectionId, PortId};

/// ABCI Query path for the IBC sub-store
pub const IBC_QUERY_PATH: &str = "store/ibc/key";


/// Path-space as listed in ICS-024
/// https://github.com/cosmos/ibc/tree/master/spec/core/ics-024-host-requirements#path-space
/// Some of these are implemented in other ICSs, but ICS-024 has a nice summary table.

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "clients/{_0}/clientType")]
pub struct ClientTypePath(pub ClientId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "clients/{_0}/clientState")]
pub struct ClientStatePath(pub ClientId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "clients/{client_id}/consensusStates/{epoch}-{height}")]
pub struct ClientConsensusStatePath {
    pub client_id: ClientId,
    pub epoch: u64,
    pub height: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "connections/{_0}")]
pub struct ConnectionsPath(pub ConnectionId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "ports/{_0}")]
pub struct PortsPath(pub PortId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "channelEnds/ports/{_0}/channels/{_1}")]
pub struct ChannelEndsPath(pub PortId, pub ChannelId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "nextSequenceSend/ports/{_0}/channels/{_1}")]
pub struct SeqSendsPath(pub PortId, pub ChannelId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "nextSequenceRecv/ports/{_0}/channels/{_1}")]
pub struct SeqRecvsPath(pub PortId, pub ChannelId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "nextSequenceAck/ports/{_0}/channels/{_1}")]
pub struct SeqAcksPath(pub PortId, pub ChannelId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "commitments/ports/{port_id}/channels/{channel_id}/sequences/{sequence}")]
pub struct CommitmentsPath {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub sequence: Sequence,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display(fmt = "acks/ports/{port_id}/channels/{channel_id}/sequences/{sequence}")]
pub struct AcksPath {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub sequence: Sequence,
}