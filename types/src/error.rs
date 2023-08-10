use std::num::ParseIntError;
use utils::encode::error::EncodeError;

use flex_error::{define_error, TraceError};
use tendermint::error::Error as TmError;

use crate::signer::SignerError;

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
            |_| { "revision height cannot be zero" },
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
            |_| { "attributes decode error" }
    }
}
