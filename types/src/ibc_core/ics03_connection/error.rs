use flex_error::{define_error, TraceError};

use crate::{ibc_core::{ics23_commitment::error::CommitmentError, ics24_host::{error::IdentifierError, identifier::{ChainId, ConnectionId}}}, signer::SignerError};

define_error! {
    ConnectionError {
        InvalidState
            { state: i32 }
            | e | { format_args!("connection state is unknown: {}", e.state) },

        ConnectionExistsAlready
            { connection_id: ConnectionId }
            | e | {
                format_args!("connection exists (was initialized) already: {0}",
                    e.connection_id)
            },

        ConnectionMismatch
            { connection_id: ConnectionId }
            | e | {
                format_args!("connection end for identifier {0} was never initialized",
                    e.connection_id)
            },
        MissingCounterparty
            | _ | { "missing counterparty" },

        InvalidIdentifier
            [ TraceError<IdentifierError> ]
            | e | { format!("identifier error: {}", e) },

        EmptyProtoConnectionEnd
            | _ | { "ConnectionEnd domain object could not be constructed out of empty proto object" },

        MissingCounterpartyPrefix
            | _ | { "missing counterparty prefix" },
        EmptyVersions
            | _ | { "empty supported versions" },
        EmptyFeatures
            | _ | { "empty supported features" },
        NoCommonVersion
            | _ | { "no common version" },
        BadPrefix
            [ TraceError<CommitmentError> ]
            | e | { format!("bad prefix: {}", e) },
        Signer
            [ SignerError ]
            | _ | { "invalid signer" },
        ChainQuery
            { chain_id: ChainId }
            |e| {
                format!("failed during a query to chain id {0}", e.chain_id)
            },
        MissingConnectionIdFromEvent
            |_| { "cannot extract connection_id from result" },
        
        MissingProofHeight
            | _ | { "missing proof height" },

        MissingConsensusHeight
            | _ | { "missing consensus height" },
        
        MissingConnectionInitEvent
            |_| { "no conn init event was in the response" },

        MissingConnectionTryEvent
            |_| { "no conn try event was in the response" },

        MissingConnectionAckEvent
            |_| { "no conn ack event was in the response" },

        MissingConnectionConfirmEvent
            |_| { "no conn confirm event was in the response" },
    }
}