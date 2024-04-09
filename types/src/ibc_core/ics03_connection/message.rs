use std::{str::FromStr, time::Duration};

use super::{connection::Counterparty, error::ConnectionError, version::Version};
use crate::{
    error::TypesError,
    ibc_core::{
        ics02_client::height::Height,
        ics23_commitment::commitment::CommitmentProofBytes,
        ics24_host::identifier::{ClientId, ConnectionId},
    },
    message::Msg,
    proofs::{ConsensusProof, Proofs},
    signer::Signer,
};
use ibc_proto::{
    google::protobuf::Any,
    ibc::core::connection::v1::{
        MsgConnectionOpenAck as RawMsgConnectionOpenAck,
        MsgConnectionOpenInit as RawMsgConnectionOpenInit,
        MsgConnectionOpenTry as RawMsgConnectionOpenTry,
    },
    Protobuf,
};

pub const OPEN_INIT_TYPE_URL: &str = "/ibc.core.connection.v1.MsgConnectionOpenInit";
pub const OPEN_TRY_TYPE_URL: &str = "/ibc.core.connection.v1.MsgConnectionOpenTry";
pub const OPEN_ACK_TYPE_URL: &str = "/ibc.core.connection.v1.MsgConnectionOpenAck";
pub const ROUTER_KEY: &str = "ibc";
///
/// Message definition `MsgConnectionOpenInit`  (i.e., the `ConnOpenInit` datagram).
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgConnectionOpenInit {
    pub client_id: ClientId,
    pub counterparty: Counterparty,
    pub version: Option<Version>,
    pub delay_period: Duration,
    pub signer: Signer,
}

impl Msg for MsgConnectionOpenInit {
    type ValidationError = TypesError;
    type Raw = RawMsgConnectionOpenInit;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_INIT_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgConnectionOpenInit> for MsgConnectionOpenInit {}

impl TryFrom<RawMsgConnectionOpenInit> for MsgConnectionOpenInit {
    type Error = ConnectionError;

    fn try_from(msg: RawMsgConnectionOpenInit) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: msg
                .client_id
                .parse()
                .map_err(ConnectionError::invalid_identifier)?,
            counterparty: msg
                .counterparty
                .ok_or_else(ConnectionError::missing_counterparty)?
                .try_into()?,
            version: msg.version.map(|version| version.try_into()).transpose()?,
            delay_period: Duration::from_nanos(msg.delay_period),
            signer: msg.signer.parse().map_err(ConnectionError::signer)?,
        })
    }
}

impl From<MsgConnectionOpenInit> for RawMsgConnectionOpenInit {
    fn from(ics_msg: MsgConnectionOpenInit) -> Self {
        RawMsgConnectionOpenInit {
            client_id: ics_msg.client_id.as_str().to_string(),
            counterparty: Some(ics_msg.counterparty.into()),
            version: ics_msg.version.map(|version| version.into()),
            delay_period: ics_msg.delay_period.as_nanos() as u64,
            signer: ics_msg.signer.to_string(),
        }
    }
}

///
/// Message definition `MsgConnectionOpenTry`  (i.e., `ConnOpenTry` datagram).
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgConnectionOpenTry {
    pub previous_connection_id: Option<ConnectionId>,
    pub client_id: ClientId,
    pub client_state: Option<Any>,
    pub counterparty: Counterparty,
    pub counterparty_versions: Vec<Version>,
    pub proofs: Proofs,
    pub delay_period: Duration,
    pub signer: Signer,
}

impl MsgConnectionOpenTry {
    /// Getter for accessing the `consensus_height` field from this message.
    /// Returns `None` if this field is not set.
    pub fn consensus_height(&self) -> Option<Height> {
        self.proofs.consensus_proof().map(|proof| proof.height())
    }
}

impl Msg for MsgConnectionOpenTry {
    type ValidationError = TypesError;
    type Raw = RawMsgConnectionOpenTry;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_TRY_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgConnectionOpenTry> for MsgConnectionOpenTry {}

impl TryFrom<RawMsgConnectionOpenTry> for MsgConnectionOpenTry {
    type Error = TypesError;

    #[allow(deprecated)]
    fn try_from(msg: RawMsgConnectionOpenTry) -> Result<Self, Self::Error> {
        let previous_connection_id = Some(msg.previous_connection_id)
            .filter(|x| !x.is_empty())
            .map(|v| FromStr::from_str(v.as_str()))
            .transpose()
            .map_err(TypesError::identifier_error)?;

        let consensus_height = msg
            .consensus_height
            .and_then(|raw_height| raw_height.try_into().ok())
            .ok_or_else(|| {
                TypesError::connection_error(ConnectionError::missing_consensus_height())
            })?;

        let consensus_proof_obj = ConsensusProof::new(
            msg.proof_consensus
                .try_into()
                .map_err(TypesError::commitment_error)?,
            consensus_height,
        )
        .map_err(TypesError::proof_error)?;

        let proof_height = msg
            .proof_height
            .and_then(|raw_height| raw_height.try_into().ok())
            .ok_or_else(|| TypesError::connection_error(ConnectionError::missing_proof_height()))?;

        let client_proof = CommitmentProofBytes::try_from(msg.proof_client)
            .map_err(TypesError::commitment_error)?;

        // Host consensus state proof can be missing for IBC-Go < 7.2.0
        let host_consensus_state_proof =
            CommitmentProofBytes::try_from(msg.host_consensus_state_proof).ok();

        let counterparty_versions = msg
            .counterparty_versions
            .into_iter()
            .map(Version::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(TypesError::connection_error)?;

        if counterparty_versions.is_empty() {
            return Err(TypesError::empty_versions());
        }

        Ok(Self {
            previous_connection_id,
            client_id: msg
                .client_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            client_state: msg.client_state,
            counterparty: msg
                .counterparty
                .ok_or_else(|| {
                    TypesError::connection_error(ConnectionError::missing_counterparty())
                })?
                .try_into()
                .map_err(TypesError::connection_error)?,
            counterparty_versions,
            proofs: Proofs::new(
                msg.proof_init
                    .try_into()
                    .map_err(TypesError::commitment_error)?,
                Some(client_proof),
                Some(consensus_proof_obj),
                host_consensus_state_proof,
                None,
                proof_height,
            )
            .map_err(TypesError::proof_error)?,
            delay_period: Duration::from_nanos(msg.delay_period),
            signer: msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgConnectionOpenTry> for RawMsgConnectionOpenTry {
    #[allow(deprecated)]
    fn from(ics_msg: MsgConnectionOpenTry) -> Self {
        RawMsgConnectionOpenTry {
            client_id: ics_msg.client_id.as_str().to_string(),
            previous_connection_id: ics_msg
                .previous_connection_id
                .map_or_else(|| "".to_string(), |v| v.as_str().to_string()),
            client_state: ics_msg.client_state,
            counterparty: Some(ics_msg.counterparty.into()),
            delay_period: ics_msg.delay_period.as_nanos() as u64,
            counterparty_versions: ics_msg
                .counterparty_versions
                .iter()
                .map(|v| v.clone().into())
                .collect(),
            proof_height: Some(ics_msg.proofs.height().into()),
            proof_init: ics_msg.proofs.object_proof().clone().into(),
            proof_client: ics_msg
                .proofs
                .client_proof()
                .map_or_else(Vec::new, |v| v.to_bytes()),
            proof_consensus: ics_msg
                .proofs
                .consensus_proof()
                .map_or_else(Vec::new, |v| v.proof().to_bytes()),
            host_consensus_state_proof: ics_msg
                .proofs
                .host_consensus_state_proof()
                .map_or_else(Vec::new, |v| v.to_bytes()),
            consensus_height: ics_msg
                .proofs
                .consensus_proof()
                .map_or_else(|| None, |h| Some(h.height().into())),
            signer: ics_msg.signer.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgConnectionOpenAck {
    pub connection_id: ConnectionId,
    pub counterparty_connection_id: ConnectionId,
    pub client_state: Option<Any>,
    pub proofs: Proofs,
    pub version: Version,
    pub signer: Signer,
}

impl MsgConnectionOpenAck {
    /// Getter for accessing the `consensus_height` field from this message.
    /// Returns `None` if this field is not set.
    pub fn consensus_height(&self) -> Option<Height> {
        self.proofs.consensus_proof().map(|proof| proof.height())
    }
}

impl Msg for MsgConnectionOpenAck {
    type ValidationError = TypesError;
    type Raw = RawMsgConnectionOpenAck;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_ACK_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgConnectionOpenAck> for MsgConnectionOpenAck {}

impl TryFrom<RawMsgConnectionOpenAck> for MsgConnectionOpenAck {
    type Error = TypesError;

    fn try_from(msg: RawMsgConnectionOpenAck) -> Result<Self, Self::Error> {
        let consensus_height = msg
            .consensus_height
            .and_then(|raw_height| raw_height.try_into().ok())
            .ok_or_else(|| {
                TypesError::connection_error(ConnectionError::missing_consensus_height())
            })?;

        let consensus_proof = ConsensusProof::new(
            msg.proof_consensus
                .try_into()
                .map_err(TypesError::commitment_error)?,
            consensus_height,
        )
        .map_err(TypesError::proof_error)?;

        let proof_height = msg
            .proof_height
            .and_then(|raw_height| raw_height.try_into().ok())
            .ok_or_else(|| TypesError::connection_error(ConnectionError::missing_proof_height()))?;

        let client_proof = CommitmentProofBytes::try_from(msg.proof_client)
            .map_err(TypesError::commitment_error)?;

        // Host consensus state proof can be missing for IBC-Go < 7.2.0
        let consensus_state_proof =
            CommitmentProofBytes::try_from(msg.host_consensus_state_proof).ok();

        Ok(Self {
            connection_id: msg
                .connection_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            counterparty_connection_id: msg
                .counterparty_connection_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            client_state: msg.client_state,
            version: msg
                .version
                .ok_or_else(TypesError::empty_versions)?
                .try_into()
                .map_err(TypesError::connection_error)?,
            proofs: Proofs::new(
                msg.proof_try
                    .try_into()
                    .map_err(TypesError::commitment_error)?,
                Some(client_proof),
                Some(consensus_proof),
                consensus_state_proof,
                None,
                proof_height,
            )
            .map_err(TypesError::proof_error)?,
            signer: msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgConnectionOpenAck> for RawMsgConnectionOpenAck {
    fn from(ics_msg: MsgConnectionOpenAck) -> Self {
        RawMsgConnectionOpenAck {
            connection_id: ics_msg.connection_id.as_str().to_string(),
            counterparty_connection_id: ics_msg.counterparty_connection_id.as_str().to_string(),
            client_state: ics_msg.client_state,
            proof_height: Some(ics_msg.proofs.height().into()),
            proof_try: ics_msg.proofs.object_proof().clone().into(),
            proof_client: ics_msg
                .proofs
                .client_proof()
                .map_or_else(Vec::new, |v| v.to_bytes()),
            proof_consensus: ics_msg
                .proofs
                .consensus_proof()
                .map_or_else(Vec::new, |v| v.proof().to_bytes()),
            consensus_height: ics_msg
                .proofs
                .consensus_proof()
                .map_or_else(|| None, |h| Some(h.height().into())),
            host_consensus_state_proof: ics_msg
                .proofs
                .host_consensus_state_proof()
                .map_or_else(Vec::new, |v| v.to_bytes()),
            version: Some(ics_msg.version.into()),
            signer: ics_msg.signer.to_string(),
        }
    }
}
