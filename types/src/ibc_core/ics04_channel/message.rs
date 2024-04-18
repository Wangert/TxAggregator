use std::str::FromStr;

use ibc_proto::{google::protobuf::field_descriptor_proto::Type, Protobuf};

use crate::{
    error::TypesError,
    ibc_core::ics24_host::identifier::{ChannelId, PortId},
    message::Msg,
    proofs::Proofs,
    signer::Signer,
};
use ibc_proto::ibc::core::channel::v1::{
    MsgChannelOpenAck as RawMsgChannelOpenAck, MsgChannelOpenConfirm as RawMsgChannelOpenConfirm,
    MsgChannelOpenInit as RawMsgChannelOpenInit, MsgChannelOpenTry as RawMsgChannelOpenTry,
};

use super::{channel::ChannelEnd, error::ChannelError, version::Version};

pub const OPEN_INIT_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenInit";
pub const OPEN_TRY_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenTry";
pub const OPEN_ACK_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenAck";
pub const OPEN_CONFIRM_TYPE_URL: &str = "/ibc.core.channel.v1.MsgChannelOpenConfirm";

pub const ROUTER_KEY: &str = "ibc";

///
/// Message definition for the first step in the channel open handshake (`ChanOpenInit` datagram).
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelOpenInit {
    pub port_id: PortId,
    pub channel: ChannelEnd,
    pub signer: Signer,
}

impl MsgChannelOpenInit {
    pub fn new(port_id: PortId, channel: ChannelEnd, signer: Signer) -> Self {
        Self {
            port_id,
            channel,
            signer,
        }
    }
}

impl Msg for MsgChannelOpenInit {
    type ValidationError = TypesError;
    type Raw = RawMsgChannelOpenInit;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_INIT_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelOpenInit> for MsgChannelOpenInit {}

impl TryFrom<RawMsgChannelOpenInit> for MsgChannelOpenInit {
    type Error = TypesError;

    fn try_from(raw_msg: RawMsgChannelOpenInit) -> Result<Self, Self::Error> {
        Ok(MsgChannelOpenInit {
            port_id: raw_msg
                .port_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            channel: raw_msg
                .channel
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_channel()))?
                .try_into()
                .map_err(TypesError::channel_error)?,
            signer: raw_msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgChannelOpenInit> for RawMsgChannelOpenInit {
    fn from(domain_msg: MsgChannelOpenInit) -> Self {
        RawMsgChannelOpenInit {
            port_id: domain_msg.port_id.to_string(),
            channel: Some(domain_msg.channel.into()),
            signer: domain_msg.signer.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelOpenTry {
    pub port_id: PortId,
    pub previous_channel_id: Option<ChannelId>,
    pub channel: ChannelEnd,
    pub counterparty_version: Version,
    pub proofs: Proofs,
    pub signer: Signer,
}

impl MsgChannelOpenTry {
    pub fn new(
        port_id: PortId,
        previous_channel_id: Option<ChannelId>,
        channel: ChannelEnd,
        counterparty_version: Version,
        proofs: Proofs,
        signer: Signer,
    ) -> Self {
        Self {
            port_id,
            previous_channel_id,
            channel,
            counterparty_version,
            proofs,
            signer,
        }
    }
}

impl Msg for MsgChannelOpenTry {
    type ValidationError = ChannelError;
    type Raw = RawMsgChannelOpenTry;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_TRY_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelOpenTry> for MsgChannelOpenTry {}

impl TryFrom<RawMsgChannelOpenTry> for MsgChannelOpenTry {
    type Error = TypesError;

    #[allow(deprecated)]
    fn try_from(raw_msg: RawMsgChannelOpenTry) -> Result<Self, Self::Error> {
        let proofs = Proofs::new(
            raw_msg
                .proof_init
                .try_into()
                .map_err(TypesError::commitment_error)?,
            None,
            None,
            None,
            None,
            raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_height()))?,
        )
        .map_err(TypesError::proof_error)?;

        let previous_channel_id = Some(raw_msg.previous_channel_id)
            .filter(|x| !x.is_empty())
            .map(|v| FromStr::from_str(v.as_str()))
            .transpose()
            .map_err(TypesError::identifier_error)?;

        let msg = MsgChannelOpenTry {
            port_id: raw_msg
                .port_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            previous_channel_id,
            channel: raw_msg
                .channel
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_channel()))?
                .try_into()
                .map_err(TypesError::channel_error)?,
            counterparty_version: raw_msg.counterparty_version.into(),
            proofs,
            signer: raw_msg.signer.parse().map_err(TypesError::signer)?,
        };

        Ok(msg)
    }
}

impl From<MsgChannelOpenTry> for RawMsgChannelOpenTry {
    #[allow(deprecated)]
    fn from(domain_msg: MsgChannelOpenTry) -> Self {
        RawMsgChannelOpenTry {
            port_id: domain_msg.port_id.to_string(),
            previous_channel_id: domain_msg
                .previous_channel_id
                .map_or_else(|| "".to_string(), |v| v.to_string()),
            channel: Some(domain_msg.channel.into()),
            counterparty_version: domain_msg.counterparty_version.to_string(),
            proof_init: domain_msg.proofs.object_proof().clone().into(),
            proof_height: Some(domain_msg.proofs.height().into()),
            signer: domain_msg.signer.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelOpenAck {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub counterparty_channel_id: ChannelId,
    pub counterparty_version: Version,
    pub proofs: Proofs,
    pub signer: Signer,
}

impl MsgChannelOpenAck {
    pub fn new(
        port_id: PortId,
        channel_id: ChannelId,
        counterparty_channel_id: ChannelId,
        counterparty_version: Version,
        proofs: Proofs,
        signer: Signer,
    ) -> Self {
        Self {
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
            proofs,
            signer,
        }
    }
}

impl Msg for MsgChannelOpenAck {
    type ValidationError = TypesError;
    type Raw = RawMsgChannelOpenAck;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_ACK_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelOpenAck> for MsgChannelOpenAck {}

impl TryFrom<RawMsgChannelOpenAck> for MsgChannelOpenAck {
    type Error = TypesError;

    fn try_from(raw_msg: RawMsgChannelOpenAck) -> Result<Self, Self::Error> {
        let proofs = Proofs::new(
            raw_msg
                .proof_try
                .try_into()
                .map_err(TypesError::commitment_error)?,
            None,
            None,
            None,
            None,
            raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_height()))?,
        )
        .map_err(TypesError::proof_error)?;

        Ok(MsgChannelOpenAck {
            port_id: raw_msg
                .port_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            channel_id: raw_msg
                .channel_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            counterparty_channel_id: raw_msg
                .counterparty_channel_id
                .parse()
                .map_err(TypesError::identifier_error)?,
            counterparty_version: raw_msg.counterparty_version.into(),
            proofs,
            signer: raw_msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgChannelOpenAck> for RawMsgChannelOpenAck {
    fn from(domain_msg: MsgChannelOpenAck) -> Self {
        RawMsgChannelOpenAck {
            port_id: domain_msg.port_id.to_string(),
            channel_id: domain_msg.channel_id.to_string(),
            counterparty_channel_id: domain_msg.counterparty_channel_id.to_string(),
            counterparty_version: domain_msg.counterparty_version.to_string(),
            proof_try: domain_msg.proofs.object_proof().clone().into(),
            proof_height: Some(domain_msg.proofs.height().into()),
            signer: domain_msg.signer.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgChannelOpenConfirm {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub proofs: Proofs,
    pub signer: Signer,
}

impl MsgChannelOpenConfirm {
    pub fn new(port_id: PortId, channel_id: ChannelId, proofs: Proofs, signer: Signer) -> Self {
        Self {
            port_id,
            channel_id,
            proofs,
            signer,
        }
    }
}

impl Msg for MsgChannelOpenConfirm {
    type ValidationError = TypesError;
    type Raw = RawMsgChannelOpenConfirm;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        OPEN_CONFIRM_TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgChannelOpenConfirm> for MsgChannelOpenConfirm {}

impl TryFrom<RawMsgChannelOpenConfirm> for MsgChannelOpenConfirm {
    type Error = TypesError;

    fn try_from(raw_msg: RawMsgChannelOpenConfirm) -> Result<Self, Self::Error> {
        let proofs = Proofs::new(
            raw_msg.proof_ack.try_into().map_err(TypesError::commitment_error)?,
            None,
            None,
            None,
            None,
            raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_height()))?,
        )
        .map_err(TypesError::proof_error)?;

        Ok(MsgChannelOpenConfirm {
            port_id: raw_msg.port_id.parse().map_err(TypesError::identifier_error)?,
            channel_id: raw_msg.channel_id.parse().map_err(TypesError::identifier_error)?,
            proofs,
            signer: raw_msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgChannelOpenConfirm> for RawMsgChannelOpenConfirm {
    fn from(domain_msg: MsgChannelOpenConfirm) -> Self {
        RawMsgChannelOpenConfirm {
            port_id: domain_msg.port_id.to_string(),
            channel_id: domain_msg.channel_id.to_string(),
            proof_ack: domain_msg.proofs.object_proof().clone().into(),
            proof_height: Some(domain_msg.proofs.height().into()),
            signer: domain_msg.signer.to_string(),
        }
    }
}
