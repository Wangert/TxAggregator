use ibc_proto::google::protobuf::Any;

use crate::signer::Signer;

#[derive(Debug, Clone)]
pub struct MsgCreateClient {
    pub client_state: Any,
    pub consensus_state: Any,
    pub signer: Signer,
}