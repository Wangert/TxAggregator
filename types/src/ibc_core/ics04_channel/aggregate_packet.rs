use crate::signer::Signer;

use super::packet::Packet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatePacket {
    pub packets: Vec<Packet>,
    pub proof: Vec<SubProof>,
    pub signer: Signer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubProof {
    pub number: u16,
    pub proof_meta_list: Vec<ProofMeta>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofMeta {
    pub hash_value: Vec<u8>,
    pub path_inner_op: InnerOp,

}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InnerOp {
    pub prefix: Vec<u8>,
    pub suffix: Vec<u8>,
}