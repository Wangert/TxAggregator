use ibc_proto::{google::protobuf::Any, Protobuf, ibc::core::client::v1::MsgUpdateClient as RawMsgUpdateClient};
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

impl Protobuf<RawMsgUpdateClient> for MsgUpdateClient {}

impl TryFrom<RawMsgUpdateClient> for MsgUpdateClient {
    type Error = TypesError;

    fn try_from(raw: RawMsgUpdateClient) -> Result<Self, Self::Error> {
        Ok(MsgUpdateClient {
            client_id: raw
                .client_id
                .parse()
                .map_err(|_| TypesError::client_id_invalid_format(raw.client_id))?,
            header: raw.client_message.ok_or_else(TypesError::header_empty)?,
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