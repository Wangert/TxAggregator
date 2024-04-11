use ibc_proto::{google::protobuf::Any, Protobuf, ibc::core::client::v1::MsgUpdateClient as RawMsgUpdateClient};
use crate::{error::TypesError, ibc_core::ics24_host::{error::IdentifierError, identifier::ClientId}, message::Msg, signer::Signer};

pub const TYPE_URL: &str = "/ibc.core.client.v1.MsgUpdateClient";
pub const ROUTER_KEY: &str = "ibc";

#[derive(Clone, Debug)]
pub struct MsgUpdateClient {
    pub client_id: ClientId,
    pub header: Any,
    pub signer: Signer,
}

impl MsgUpdateClient {
    pub fn new(client_id: ClientId, header: Any, signer: Signer) -> Self {
        MsgUpdateClient {
            client_id,
            header,
            signer,
        }
    }
}

impl Msg for MsgUpdateClient {
    type ValidationError = TypesError;
    type Raw = RawMsgUpdateClient;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgUpdateClient> for MsgUpdateClient {}

impl TryFrom<RawMsgUpdateClient> for MsgUpdateClient {
    type Error = TypesError;

    fn try_from(raw: RawMsgUpdateClient) -> Result<Self, Self::Error> {
        Ok(MsgUpdateClient {
            client_id: raw
                .client_id
                .parse()
                .map_err(TypesError::ics24_host)?,
            header: raw.client_message.ok_or_else(TypesError::raw_msg_update_client_header_empty)?,
            signer: raw.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgUpdateClient> for RawMsgUpdateClient {
    fn from(ics_msg: MsgUpdateClient) -> Self {
        RawMsgUpdateClient {
            client_id: ics_msg.client_id.to_string(),
            client_message: Some(ics_msg.header),
            signer: ics_msg.signer.to_string(),
        }
    }
}