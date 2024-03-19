use derive_more::Display;

use super::identifier::ClientId;

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