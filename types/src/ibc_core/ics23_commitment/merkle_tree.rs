use ics23::CommitmentProof;
use ibc_proto::ibc::core::commitment::v1::MerkleProof as IbcMerkleProof;
use tendermint::merkle::proof::ProofOps as TendermintProof;

use crate::error::TypesError;

#[derive(Clone, Debug, PartialEq)]
pub struct MerkleProof {
    pub proofs: Vec<CommitmentProof>,
}

/// Convert to ics23::CommitmentProof
impl From<IbcMerkleProof> for MerkleProof {
    fn from(proof: IbcMerkleProof) -> Self {
        Self {
            proofs: proof.proofs,
        }
    }
}

impl From<MerkleProof> for IbcMerkleProof {
    fn from(proof: MerkleProof) -> Self {
        Self {
            proofs: proof.proofs,
        }
    }
}

pub fn tendermint_proof_to_ics_merkle_proof(tendermint_proof: &TendermintProof) -> Result<MerkleProof, TypesError> {
    let mut proofs = Vec::new();

    for op in &tendermint_proof.ops {
        let mut parsed = CommitmentProof { proof: None };

        prost::Message::merge(&mut parsed, op.data.as_slice())
            .map_err(TypesError::commitment_proof_decoding_failed)?;

        proofs.push(parsed);
    }

    Ok(MerkleProof::from(IbcMerkleProof { proofs }))
}