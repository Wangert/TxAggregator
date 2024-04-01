use std::time::Duration;

use crate::{error::TypesError, ibc_core::ics24_host::identifier::ClientId, message::Msg, signer::Signer};
use ibc_proto::{ibc::core::connection::v1::MsgConnectionOpenInit as RawMsgConnectionOpenInit, Protobuf};
use super::{connection::Counterparty, error::ConnectionError, version::Version};

pub const TYPE_URL: &str = "/ibc.core.connection.v1.MsgConnectionOpenInit";
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
        TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgConnectionOpenInit> for MsgConnectionOpenInit {}

impl TryFrom<RawMsgConnectionOpenInit> for MsgConnectionOpenInit {
    type Error = ConnectionError;

    fn try_from(msg: RawMsgConnectionOpenInit) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: msg.client_id.parse().map_err(ConnectionError::invalid_identifier)?,
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