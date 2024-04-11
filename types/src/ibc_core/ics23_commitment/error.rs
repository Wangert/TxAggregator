use flex_error::{define_error, TraceError};
use prost::DecodeError;
use utils::encode::error::EncodeError;

define_error! {
    CommitmentError {
        ZeroHeight
            | _ | { format_args!("proof height cannot be zero") },

        EmptyProof
            | _ | { format_args!("proof cannot be empty") },

        Encode
            [ TraceError<EncodeError> ]
            | _ | { "protobuf encode error" },

        InvalidRawMerkleProof
            [ TraceError<DecodeError> ]
            |_| { "invalid raw merkle proof" },

        CommitmentProofDecodingFailed
            [ TraceError<DecodeError> ]
            |_| { "failed to decode commitment proof" },
        
        EmptyCommitmentPrefix
            |_| { "empty commitment prefix" },

        EmptyMerkleProof
            |_| { "empty merkle proof" },

        EmptyMerkleRoot
            |_| { "empty merkle root" },

        EmptyVerifiedValue
            |_| { "empty verified value" },

        NumberOfSpecsMismatch
            |_| { "mismatch between the number of proofs with that of specs" },

        NumberOfKeysMismatch
            |_| { "mismatch between the number of proofs with that of keys" },

        InvalidMerkleProof
            |_| { "invalid merkle proof" },

        VerificationFailure
            |_| { "proof verification failed" }
    }
}