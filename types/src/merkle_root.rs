use ibc_proto::google::protobuf::Any;
use ibc_proto::Protobuf;

use crate::error::TypesError;
use crate::ibc_core::ics04_channel::message::ROUTER_KEY;
use crate::message::Msg;
use crate::{ibc_core::ics04_channel::packet::Packet, signer::Signer};
use crate::proto::{merkle_root::MsgSetHashValue as RawMsgSetHashValue, aggregate_packet::Packet as RawPacket, height::Height as RawHeight};

pub const SET_HASH_VALUE_TYPE_URL: &str = "/ibc.core.channel.v1.MsgSetHashValue";

#[derive(Debug, Clone)]
pub struct MsgSetHashValue {
    pub key: Packet,
    pub value: Vec<u8>,
    pub signer: Signer,
}

impl MsgSetHashValue {
    pub fn new(key: Packet, value: Vec<u8>, signer: Signer) -> Self {
        MsgSetHashValue {
            key,
            value,
            signer,
        }
    }
}

impl Msg for MsgSetHashValue {
    type ValidationError = TypesError;
    type Raw = RawMsgSetHashValue;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        SET_HASH_VALUE_TYPE_URL.to_string()
    }
}

impl From<MsgSetHashValue> for RawMsgSetHashValue {
    fn from(msg_set_hash_value: MsgSetHashValue) -> Self {
        let t_height = RawHeight {
            revision_number: msg_set_hash_value.key.timeout_height.commitment_revision_number(),
            revision_height: msg_set_hash_value.key.timeout_height.commitment_revision_height(),
        };

        RawMsgSetHashValue {
            key: Some(RawPacket {
                sequence: msg_set_hash_value.key.sequence.as_u64(),
                source_port: msg_set_hash_value.key.source_port.to_string(),
                source_channel: msg_set_hash_value.key.source_channel.to_string(),
                destination_port: msg_set_hash_value.key.destination_port.to_string(),
                destination_channel: msg_set_hash_value.key.destination_channel.to_string(),
                data: msg_set_hash_value.key.data.clone(),
                timeout_height: Some(t_height),
                timeout_timestamp: msg_set_hash_value.key.timeout_timestamp.nanoseconds(),
            }),
            value: msg_set_hash_value.value,
            signer: msg_set_hash_value.signer.to_string(),
        }
    }
}