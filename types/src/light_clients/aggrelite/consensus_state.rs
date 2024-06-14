use ibc::core::commitment_types::commitment::CommitmentRoot;
use ibc_proto::google::protobuf::Any;
use ibc_proto::ibc::lightclients::tendermint::v1::ConsensusState as TmConsensusState;
use ibc_proto::Protobuf;
use serde::{Deserialize, Serialize};
use tendermint::{Hash, time::Time, hash::Algorithm};
use tendermint_proto::google::protobuf as TmProtobuf;
use prost::Message;

use crate::error::TypesError;

pub const AGGRELITE_CONSENSUS_STATE_TYPE_URL: &str = "/ibc.lightclients.aggrelite.v1.ConsensusState";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusState {
    pub timestamp: Time,
    pub commitment_root: CommitmentRoot,
    pub next_validators_hash: Hash,
}

impl ConsensusState {
    pub fn new(
        commitment_root: CommitmentRoot,
        timestamp: Time,
        next_validators_hash: Hash,
    ) -> Self {
        Self {
            timestamp,
            commitment_root,
            next_validators_hash,
        }
    }

    fn root(&self) -> &CommitmentRoot {
        &self.commitment_root
    }

    fn timestamp(&self) -> Time {
        self.timestamp
    }
}

impl Protobuf<TmConsensusState> for ConsensusState {}

impl TryFrom<TmConsensusState> for ConsensusState {
    type Error = TypesError;

    fn try_from(tm_consensus_state: TmConsensusState) -> Result<Self, Self::Error> {
        let ibc_proto::google::protobuf::Timestamp { seconds, nanos } = tm_consensus_state
            .timestamp
            .ok_or_else(|| TypesError::consensus_state("missing timestamp".into()))?;
        // FIXME: shunts like this are necessary due to
        let proto_timestamp = TmProtobuf::Timestamp { seconds, nanos };
        let timestamp = proto_timestamp
            .try_into()
            .map_err(|e| TypesError::invalid_consensus_state_timestamp(format!("{e}")))?;

        Ok(Self {
            commitment_root: tm_consensus_state
                .root
                .ok_or_else(|| TypesError::consensus_state("missing commitment root".into()))?
                .hash
                .into(),
            timestamp,
            next_validators_hash: Hash::from_bytes(
                Algorithm::Sha256,
                &tm_consensus_state.next_validators_hash,
            )
            .map_err(|e| TypesError::tendermint_hash(e))?,
        })
    }
}

impl From<ConsensusState> for TmConsensusState {
    fn from(value: ConsensusState) -> Self {
        let TmProtobuf::Timestamp { seconds, nanos } = value.timestamp.into();
        let timestamp = ibc_proto::google::protobuf::Timestamp { seconds, nanos };

        TmConsensusState {
            timestamp: Some(timestamp),
            root: Some(ibc_proto::ibc::core::commitment::v1::MerkleRoot {
                hash: value.commitment_root.into_vec(),
            }),
            next_validators_hash: value.next_validators_hash.as_bytes().to_vec(),
        }
    }
}

impl Protobuf<Any> for ConsensusState {}

impl TryFrom<Any> for ConsensusState {
    type Error = TypesError;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        use bytes::Buf;
        use core::ops::Deref;

        fn decode_consensus_state<B: Buf>(buf: B) -> Result<ConsensusState, TypesError> {
            TmConsensusState::decode(buf)
                .map_err(|e| TypesError::tendermint_consensus_state_decode(e))?
                .try_into()
        }

        match raw.type_url.as_str() {
            TENDERMINT_CONSENSUS_STATE_TYPE_URL => {
                decode_consensus_state(raw.value.deref()).map_err(Into::into)
            }
            _ => Err(TypesError::unknown_consensus_state_type(raw.type_url)),
        }
    }
}

impl From<ConsensusState> for Any {
    fn from(consensus_state: ConsensusState) -> Self {
        Any {
            type_url: AGGRELITE_CONSENSUS_STATE_TYPE_URL.to_string(),
            value: Protobuf::<TmConsensusState>::encode_vec(consensus_state),
        }
    }
}

impl From<tendermint::block::Header> for ConsensusState {
    fn from(header: tendermint::block::Header) -> Self {
        Self {
            commitment_root: CommitmentRoot::from_bytes(header.app_hash.as_ref()),
            timestamp: header.time,
            next_validators_hash: header.next_validators_hash,
        }
    }
}

