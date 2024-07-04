use super::{aggrelite, ics07_tendermint};

#[derive(Debug, Clone)]
pub enum ClientStateType {
    Tendermint(ics07_tendermint::client_state::ClientState),
    Aggrelite(aggrelite::client_state::ClientState)
}

#[derive(Debug, Clone)]
pub enum ConsensusStateType {
    Tendermint(ics07_tendermint::consensus_state::ConsensusState),
    Aggrelite(aggrelite::consensus_state::ConsensusState)
}

#[derive(Debug, Clone)]
pub enum ClientType {
    Tendermint,
    Aggrelite
}