use ibc_proto::{google::protobuf::Any, protobuf::Protobuf, ibc::core::client::v1::MsgUpdateClient as IbcMsgUpdateClient};
use crate::{ibc_core::ics24_host::identifier::ClientId, signer::Signer, error::TypesError};

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

impl Protobuf<IbcMsgUpdateClient> for MsgUpdateClient {}

impl TryFrom<IbcMsgUpdateClient> for MsgUpdateClient {
    type Error = TypesError;

    fn try_from(raw: IbcMsgUpdateClient) -> Result<Self, Self::Error> {
        Ok(MsgUpdateClient {
            client_id: raw
                .client_id
                .parse()
                .map_err(|_| TypesError::client_id_invalid_format(raw.client_id))?,
            header: raw.header.ok_or_else(TypesError::header_empty)?,
            signer: raw.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<MsgUpdateClient> for IbcMsgUpdateClient {
    fn from(ics_msg: MsgUpdateClient) -> Self {
        IbcMsgUpdateClient {
            client_id: ics_msg.client_id.to_string(),
            header: Some(ics_msg.header),
            signer: ics_msg.signer.to_string(),
        }
    }
}