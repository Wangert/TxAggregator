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

impl AggregatePacket {
    pub fn new(packets: Vec<Packet>, proof: Vec<SubProof>, signer: Signer) -> Self {
        Self {
            packets,
            proof,
            signer,
        }
    }
}

impl SubProof {
    pub fn new(number: u16, proof_meta_list: Vec<ProofMeta>) -> Self {
        Self {
            number,
            proof_meta_list,
        }
    }
}

impl ProofMeta {
    pub fn new(hash_value: Vec<u8>, path_inner_op: InnerOp) -> Self {
        Self { hash_value, path_inner_op }
    }
}