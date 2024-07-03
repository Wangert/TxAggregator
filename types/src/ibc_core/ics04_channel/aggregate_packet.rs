use ibc_proto::Protobuf;
use ics23::InnerOp;

use crate::error::TypesError;
use crate::message::Msg;
use crate::proto::aggregate_packet::{
    AggregatePacket as RawAggregatePacket, InnerOp as RawInnerOp, Packet as RawPacket,
    ProofMeta as RawProofMeta, SubProof as RawSubProof,
};

use crate::proto::height::Height as RawHeight;
use crate::{ibc_core::ics02_client::height::Height, signer::Signer};

use super::message::ROUTER_KEY;
use super::packet::Packet;

pub const AGGREGATE_PACKET_TYPE_URL: &str = "/ibc.core.channel.v1.MsgAggregatePacket";

#[derive(Clone, Debug)]
pub struct AggregatePacket {
    pub packets: Vec<Packet>,
    pub packets_leaf_number: Vec<u64>,
    pub proof: Vec<SubProof>,
    pub signer: Signer,
    pub height: Height,
}

impl Msg for AggregatePacket {
    type ValidationError = TypesError;
    type Raw = RawAggregatePacket;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        AGGREGATE_PACKET_TYPE_URL.to_string()
    }
}

impl From<AggregatePacket> for RawAggregatePacket {
    fn from(value: AggregatePacket) -> Self {
        let height = RawHeight {
            revision_number: value.height.revision_number(),
            revision_height: value.height.revision_height(),
        };

        RawAggregatePacket {
            proof: value.proof.iter().map(|sp| sp.clone().into()).collect(),
            signer: value.signer.to_string(),
            packets: value
                .packets
                .iter()
                .map(|p| {
                    let t_height = RawHeight {
                        revision_number: p.timeout_height.commitment_revision_number(),
                        revision_height: p.timeout_height.commitment_revision_height(),
                    };
                    RawPacket {
                        sequence: p.sequence.as_u64(),
                        source_port: p.source_port.to_string(),
                        source_channel: p.source_channel.to_string(),
                        destination_port: p.destination_port.to_string(),
                        destination_channel: p.destination_channel.to_string(),
                        data: p.data.clone(),
                        timeout_height: Some(t_height),
                        timeout_timestamp: p.timeout_timestamp.nanoseconds(),
                    }
                })
                .collect(),
            height: Some(height),
            packets_leaf_number: value.packets_leaf_number,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SubProof {
    pub number: u64,
    pub proof_meta_list: Vec<ProofMeta>,
}

impl From<SubProof> for RawSubProof {
    fn from(value: SubProof) -> Self {
        RawSubProof {
            number: value.number,
            proof_meta_list: value
                .proof_meta_list
                .iter()
                .map(|pm| pm.clone().into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProofMeta {
    pub hash_value: Vec<u8>,
    pub path_inner_op: InnerOp,
}

impl From<ProofMeta> for RawProofMeta {
    fn from(value: ProofMeta) -> Self {
        let raw_inner_op = RawInnerOp {
            hash: value.path_inner_op.hash,
            prefix: value.path_inner_op.prefix,
            suffix: value.path_inner_op.suffix,
        };
        RawProofMeta {
            path_inner_op: Some(raw_inner_op),
            hash_value: value.hash_value,
        }
    }
}

// #[derive(Clone, Debug, PartialEq, Eq)]
// pub struct InnerOp {
//     pub prefix: Vec<u8>,
//     pub suffix: Vec<u8>,
// }

impl AggregatePacket {
    pub fn new(
        packets: Vec<Packet>,
        packets_leaf_number: Vec<u64>,
        proof: Vec<SubProof>,
        signer: Signer,
        height: Height,
    ) -> Self {
        Self {
            packets,
            packets_leaf_number,
            proof,
            signer,
            height,
        }
    }
}

impl SubProof {
    pub fn new(number: u64, proof_meta_list: Vec<ProofMeta>) -> Self {
        Self {
            number,
            proof_meta_list,
        }
    }
}

impl ProofMeta {
    pub fn new(hash_value: Vec<u8>, path_inner_op: InnerOp) -> Self {
        Self {
            hash_value,
            path_inner_op,
        }
    }
}
