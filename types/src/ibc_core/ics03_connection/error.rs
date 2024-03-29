use flex_error::{define_error, TraceError};

use crate::ibc_core::{ics23_commitment::error::CommitmentError, ics24_host::{error::IdentifierError, identifier::ConnectionId}};

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
    }
}