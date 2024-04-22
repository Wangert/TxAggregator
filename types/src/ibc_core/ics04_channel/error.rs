use flex_error::{define_error, TraceError};

use crate::{ibc_core::ics24_host::{error::IdentifierError, identifier::{ChannelId, PortId}}, timestamp::ParseTimestampError};

use tendermint_proto::Error as TendermintError;

define_error! {
    ChannelError {
        MissingChannelId
        | _ | { "missing channel id" },
        UnknownState
            { state: i32 }
            | e | { format_args!("channel state unknown: {}", e.state) },

        Identifier
            [ IdentifierError ]
            | _ | { "identifier error" },

        UnknownOrderType
            { type_id: String }
            | e | { format_args!("channel order type unknown: {}", e.type_id) },

        InvalidConnectionHopsLength
            { expected: usize, actual: usize }
            | e | {
                format_args!(
                    "invalid connection hops length: expected {0}; actual {1}",
                    e.expected, e.actual)
            },

        InvalidPacketCounterparty
            { port_id: PortId, channel_id: ChannelId }
            | e | {
                format_args!(
                    "packet destination port {} and channel {} doesn't match the counterparty's port/channel",
                    e.port_id, e.channel_id)
            },

        InvalidVersion
            [ TraceError<TendermintError> ]
            | _ | { "invalid version" },

        MissingHeight
            | _ | { "invalid proof: missing height" },

        MissingNextRecvSeq
            { port_id: PortId, channel_id: ChannelId }
        | e | {
                format_args!("Missing sequence number for receiving packets on port {0} and channel {1}",
                             e.port_id,
                             e.channel_id)
            },

        ZeroPacketSequence
            | _ | { "packet sequence cannot be 0" },

        ZeroPacketData
            | _ | { "packet data bytes cannot be empty" },

        InvalidTimeoutHeight
            | _ | { "invalid timeout height for the packet" },

        InvalidPacket
            | _ | { "invalid packet" },

        MissingPacket
            | _ | { "there is no packet in this message" },

        MissingCounterparty
            | _ | { "missing counterparty" },

        NoCommonVersion
            | _ | { "no commong version" },

        MissingChannel
            | _ | { "missing channel end" },
        MissingConnectionId 
            | _ | { "missing connection id" },

        MissingChannelIdFromEvent
            |_| { "cannot extract channel_id from result" },
        MissingVersion
            |_| { "missing channel version" },

        MissingChannelInitEvent
            |_| { "no conn init event was in the response" },

        MissingChannelTryEvent
            |_| { "no conn try event was in the response" },

        MissingChannelAckEvent
            |_| { "no conn ack event was in the response" },

        MissingChannelConfirmEvent
            |_| { "no conn confirm event was in the response" },
        
        MismatchPort
            { payload: String }
            |e| { format!("{} process port mismatch", e.payload) },
        ChannelAlreadyExist
            { channel_id: ChannelId }
            |e| { format_args!("channel '{}' already exist in an incompatible state", e.channel_id) },
        MissingChannelOnTarget
            |_| { "missing channel on target chain" },

        InvalidPacketTimestamp
            [ ParseTimestampError ]
            | _ | { "Invalid packet timeout timestamp value" },
    }
}