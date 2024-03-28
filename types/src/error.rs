use std::num::ParseIntError;
use subtle_encoding::Error as SubtleError;
use utils::encode::error::EncodeError;

use crate::{ibc_core::ics04_channel::error::ChannelError, signer::SignerError};
use flex_error::{define_error, TraceError};
use tendermint::error::Error as TmError;
use tendermint_proto::Error as TendermintProtoError;

define_error! {
    TypesError {
        // ics07_client_state
        UnknownClientStateType
            { client_state_type: String }
            |e| { format_args!("unknown client state type: {0}", e.client_state_type) },
        TendermintClientStateDecode
            [ TraceError<prost::DecodeError> ]
            |_| { "tendermint client state protobuf decode error" },
        InvalidTrustLevel
            { numerator: u64, denominator: u64 }
            |e| { format_args!("invalid trust threshold: {}/{}", e.numerator, e.denominator) },
        TrustedHeight
            { payload: String }
            |e| { format_args!("{} trusted height", e.payload) },
        TrustingPeriod
            { payload: String }
            |e| { format_args!("{} trusted period", e.payload) },
        UnbondingPeriod
            { payload: String }
            |e| { format_args!("{} unbonding period", e.payload) },
        TrustLevel
            { payload: String }
            |e| { format_args!("{} trust level", e.payload) },
        MaxClockDrift
            { payload: String }
            |e| { format_args!("{} max clock drift", e.payload) },
        LatestHeight
            { payload: String }
            |e| { format_args!("{} latest height", e.payload) },
        // ics07_consensus_state
        UnknownConsensusStateType
            { consensus_state_type: String }
            |e| { format_args!("unknown consensus state type: {0}", e.consensus_state_type) },
        TendermintConsensusStateDecode
            [ TraceError<prost::DecodeError> ]
            |_| { "tendermint consensus state protobuf decode error" },
        ConsensusState
            { payload: String }
            |e| { format_args!("consensus state: {}", e.payload) },
        InvalidConsensusStateTimestamp
            { payload: String }
            |e| { format!("invalid consensus state timestamp: {}", e.payload) },
        TendermintHash
            [ TraceError<TmError> ]
            |_| { "tendermint hash error" },

        // height
        InvalidHeight
            { height: String }
            |e| { format_args!("invalid height {0}", e.height) },
        InvalidRawHeader
            [ TraceError<TendermintProtoError> ]
            | _ | { "invalid raw header" },
        InvalidRawHeaderSet
            [ TraceError<TmError> ]
            | _ | { "invalid raw header" },
        InvalidHeightObject
            { height: String }
            |e| { format_args!("invalid height {0}", e.height) },
        InvalidHeightResult
            | _ | { "height cannot end up zero or negative" },
        HeightConversion
            { height: String }
            [ TraceError<ParseIntError> ]
            |e| { format_args!("cannot convert into a `Height` type from string {0}", e.height) },
        ZeroHeight
            |_| { "attempted to parse invalid height 0-0" },
        UnknownHeaderType
            { header_type: String }
            | e | {
                format_args!("unknown header type: {0}",
                    e.header_type)
            },

        // ics07_tendermint
        InvalidTrustingPeriod
            { reason: String }
            |e| { format_args!("invalid trusting period: {}", e.reason) },
        InvalidUnbondingPeriod
            { reason: String }
            |e| { format_args!("invalid unbonding period: {}", e.reason) },
        InvalidProofSpecs
            { reason: String }
            |e| { format_args!("invalid proof specs: {}", e.reason) },
        EmptyClientState
            |_| { "empty client state" },
        EmptyConsensusState
            |_| { "empty consensus state" },
        Signer
            [ SignerError ]
            | _ | { "failed to parse signer" },

        // ics23_commitment
        CommitmentProofDecodingFailed
            [ TraceError<prost::DecodeError> ]
            |_| { "failed to decode commitment proof" },

        // ics24_host identifier
        IdContainSeparator
            { id : String }
            | e | { format_args!("identifier {0} cannot contain separator '/'", e.id) },
        IdInvalidLength
            {
                id: String,
                length: usize,
                min: usize,
                max: usize,
            }
            |e| { format_args!("identifier {0} has invalid length {1} must be between {2}-{3} characters", e.id, e.length, e.min, e.max) },
        IdInvalidCharacter
            { id: String }
            |e| { format_args!("identifier {0} must only contain alphanumeric characters or `.`, `_`, `+`, `-`, `#`, - `[`, `]`, `<`, `>`", e.id) },
        IdEmpty
            |_| { "identifier cannot be empty" },
        HeaderEmpty
            |_| { "empty block header" },
        ChainIdInvalidFormat
            { id: String }
            |e| { format_args!("chain identifiers are expected to be in epoch format {0}", e.id) },
        ClientIdInvalidFormat
            { id: String }
            |e| { format_args!("client identifiers are expected to be in epoch format {0}", e.id) },
        UnknownClientType
            { client_type: String }
            |e| { format_args!("unknown client type: {0}", e.client_type) },
        InvalidCounterpartyChannelId
            |_| { "Invalid channel id in counterparty" },

        // ibc_events
        IncorrectEventType
            { event: String }
            |e| { format_args!("incorrect event type: {}", e.event) },
        UnsupportedAbciEvent
            { event_type: String}
            |e| { format_args!("Unable to parse abci event type '{}' into IbcEvent", e.event_type)},
        AttributesDecode
            [ TraceError<EncodeError> ]
            |_| { "attributes decode error" },

        // ics04_channel
        InvalidStringAsSequence
            { value: String }
            [ TraceError<core::num::ParseIntError> ]
            | e | {
                format_args!(
                    "String {0} cannot be converted to packet sequence",
                    e.value)
            },
        ChannelError
            [ ChannelError ]
            | _ | { "channel error" },

        // ics03_connection
        ConnectionInvalidIdentifier
            | _ | { "connection invalid identifier" },

        // other
        ProtobufDecode
            [ TraceError<prost::DecodeError> ]
            | _ | { "protobuf decode error" },

        MissingValidatorSet
            |_| { "missing validator set" },

        MissingTrustedValidatorSet
            |_| { "missing trusted validator set" },

        MissingTrustedHeight
            |_| { "missing trusted height" },

        MissingTrustingPeriod
            |_| { "missing trusting period" },

        MissingUnbondingPeriod
            |_| { "missing unbonding period" },

        MissingTrustThreshold
            |_| { "missing trust threshold" },

        MissingSignedHeader
            |_| { "missing signed header" },
            InvalidHeader
            { reason: String }
            [ TmError ]
            |e| { format_args!("invalid header, failed basic validation: {}", e.reason) },
        MismatchedRevisions
            {
                current_revision: u64,
                update_revision: u64,
            }
            |e| {
                format_args!("the header's current/trusted revision number ({0}) and the update's revision number ({1}) should be the same", e.current_revision, e.update_revision)
            },
        HexDecode
            [ TraceError<SubtleError> ]
            |e| { format!("hex bytes decode error: {}", e) },
        AbciEventMissingRawHeader
            |_| { "abic_event miss raw header" }
    }
}
