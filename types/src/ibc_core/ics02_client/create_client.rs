use ibc_proto::{google::protobuf::Any, protobuf::Protobuf, ibc::core::client::v1::MsgCreateClient as IbcMsgCreateClient};

use crate::{signer::Signer, error::TypesError};

#[derive(Debug, Clone)]
pub struct MsgCreateClient {
    pub client_state: Any,
    pub consensus_state: Any,
    pub signer: Signer,
}

impl MsgCreateClient {
    pub fn new(client_state: Any, consensus_state: Any, signer: Signer) -> Self {
        MsgCreateClient {
            client_state,
            consensus_state,
            signer,
        }
    }
}

impl Protobuf<IbcMsgCreateClient> for MsgCreateClient {}

impl TryFrom<IbcMsgCreateClient> for MsgCreateClient {
    type Error = TypesError;

    fn try_from(raw: IbcMsgCreateClient) -> Result<Self, TypesError> {
        let raw_client_state = raw
            .client_state
            .ok_or_else(TypesError::empty_client_state)?;

        let raw_consensus_state = raw
            .consensus_state
            .ok_or_else(TypesError::empty_client_state)?;

        Ok(MsgCreateClient::new(
            raw_client_state,
            raw_consensus_state,
            raw.signer.parse().map_err(TypesError::signer)?,
        ))
    }
}

impl From<MsgCreateClient> for IbcMsgCreateClient {
    fn from(msg_create_client: MsgCreateClient) -> Self {
        IbcMsgCreateClient {
            client_state: Some(msg_create_client.client_state),
            consensus_state: Some(msg_create_client.consensus_state),
            signer: msg_create_client.signer.to_string(),
        }
    }
}