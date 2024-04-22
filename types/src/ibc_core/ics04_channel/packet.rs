use std::str::FromStr;

use ibc_proto::Protobuf;
use serde::{Deserialize, Serialize};

use ibc_proto::ibc::core::channel::v1::MsgRecvPacket as RawMsgRecvPacket;
use ibc_proto::ibc::core::channel::v1::Packet as RawPacket;

use crate::timestamp::Expiry::Expired;
use crate::{
    error::TypesError,
    ibc_core::{
        ics02_client::height::Height,
        ics24_host::identifier::{ChannelId, PortId},
    },
    message::Msg,
    proofs::Proofs,
    signer::Signer,
    timestamp::Timestamp,
};

use super::{error::ChannelError, message::ROUTER_KEY, timeout::TimeoutHeight};

pub const TYPE_URL: &str = "/ibc.core.channel.v1.MsgRecvPacket";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecvPacket {
    pub packet: Packet,
    pub proofs: Proofs,
    pub signer: Signer,
}

impl RecvPacket {
    pub fn new(packet: Packet, proofs: Proofs, signer: Signer) -> Self {
        Self {
            packet,
            proofs,
            signer,
        }
    }
}

impl Msg for RecvPacket {
    type ValidationError = TypesError;
    type Raw = RawMsgRecvPacket;

    fn route(&self) -> String {
        ROUTER_KEY.to_string()
    }

    fn type_url(&self) -> String {
        TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgRecvPacket> for RecvPacket {}

impl TryFrom<RawMsgRecvPacket> for RecvPacket {
    type Error = TypesError;

    fn try_from(raw_msg: RawMsgRecvPacket) -> Result<Self, Self::Error> {
        let proofs = Proofs::new(
            raw_msg
                .proof_commitment
                .try_into()
                .map_err(TypesError::commitment_error)?,
            None,
            None,
            None,
            None,
            raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_height()))?,
        )
        .map_err(TypesError::proof_error)?;

        Ok(RecvPacket {
            packet: raw_msg
                .packet
                .ok_or_else(|| TypesError::channel_error(ChannelError::missing_packet()))?
                .try_into()?,
            proofs,
            signer: raw_msg.signer.parse().map_err(TypesError::signer)?,
        })
    }
}

impl From<RecvPacket> for RawMsgRecvPacket {
    fn from(recv_packet: RecvPacket) -> Self {
        RawMsgRecvPacket {
            packet: Some(recv_packet.packet.into()),
            proof_commitment: recv_packet.proofs.object_proof().clone().into(),
            proof_height: Some(recv_packet.proofs.height().into()),
            signer: recv_packet.signer.to_string(),
        }
    }
}

#[derive(Clone, Default, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct Packet {
    pub sequence: Sequence,
    pub source_port: PortId,
    pub source_channel: ChannelId,
    pub destination_port: PortId,
    pub destination_channel: ChannelId,
    #[serde(serialize_with = "crate::serializers::ser_hex_upper")]
    pub data: Vec<u8>,
    pub timeout_height: TimeoutHeight,
    pub timeout_timestamp: Timestamp,
}

struct PacketData<'a>(&'a [u8]);

impl<'a> core::fmt::Debug for PacketData<'a> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(formatter, "{:?}", self.0)
    }
}

impl core::fmt::Debug for Packet {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        // Remember: if you alter the definition of `Packet`,
        // 1. update the formatter debug struct builder calls (return object of
        //    this function)
        // 2. update this destructuring assignment accordingly
        let Packet {
            sequence: _,
            source_port: _,
            source_channel: _,
            destination_port: _,
            destination_channel: _,
            data,
            timeout_height: _,
            timeout_timestamp: _,
        } = self;
        let data_wrapper = PacketData(data);

        formatter
            .debug_struct("Packet")
            .field("sequence", &self.sequence)
            .field("source_port", &self.source_port)
            .field("source_channel", &self.source_channel)
            .field("destination_port", &self.destination_port)
            .field("destination_channel", &self.destination_channel)
            .field("data", &data_wrapper)
            .field("timeout_height", &self.timeout_height)
            .field("timeout_timestamp", &self.timeout_timestamp)
            .finish()
    }
}

impl Packet {
    pub fn timed_out(&self, dst_chain_ts: &Timestamp, dst_chain_height: Height) -> bool {
        let height_timed_out = self.timeout_height.has_expired(dst_chain_height);

        let timestamp_timed_out = self.timeout_timestamp != Timestamp::none()
            && dst_chain_ts.check_expiry(&self.timeout_timestamp) == Expired;

        height_timed_out || timestamp_timed_out
    }
}

/// Custom debug output to omit the packet data
impl core::fmt::Display for Packet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "seq:{}, path:{}/{}->{}/{}, toh:{}, tos:{})",
            self.sequence,
            self.source_channel,
            self.source_port,
            self.destination_channel,
            self.destination_port,
            self.timeout_height,
            self.timeout_timestamp
        )
    }
}

impl TryFrom<RawPacket> for Packet {
    type Error = TypesError;

    fn try_from(raw_pkt: RawPacket) -> Result<Self, Self::Error> {
        if Sequence::from(raw_pkt.sequence).is_zero() {
            return Err(TypesError::channel_error(
                ChannelError::zero_packet_sequence(),
            ));
        }

        let packet_timeout_height: TimeoutHeight = raw_pkt.timeout_height.try_into()?;

        if raw_pkt.data.is_empty() {
            return Err(TypesError::channel_error(ChannelError::zero_packet_data()));
        }

        let timeout_timestamp = Timestamp::from_nanoseconds(raw_pkt.timeout_timestamp)
            .map_err(|e| TypesError::channel_error(ChannelError::invalid_packet_timestamp(e)))?;

        Ok(Packet {
            sequence: Sequence::from(raw_pkt.sequence),
            source_port: raw_pkt
                .source_port
                .parse()
                .map_err(TypesError::identifier_error)?,
            source_channel: raw_pkt
                .source_channel
                .parse()
                .map_err(TypesError::identifier_error)?,
            destination_port: raw_pkt
                .destination_port
                .parse()
                .map_err(TypesError::identifier_error)?,
            destination_channel: raw_pkt
                .destination_channel
                .parse()
                .map_err(TypesError::identifier_error)?,
            data: raw_pkt.data,
            timeout_height: packet_timeout_height,
            timeout_timestamp,
        })
    }
}

impl From<Packet> for RawPacket {
    fn from(packet: Packet) -> Self {
        RawPacket {
            sequence: packet.sequence.0,
            source_port: packet.source_port.to_string(),
            source_channel: packet.source_channel.to_string(),
            destination_port: packet.destination_port.to_string(),
            destination_channel: packet.destination_channel.to_string(),
            data: packet.data,
            timeout_height: packet.timeout_height.into(),
            timeout_timestamp: packet.timeout_timestamp.nanoseconds(),
        }
    }
}

/// The sequence number of a packet enforces ordering among packets from the same source.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Sequence(u64);

impl FromStr for Sequence {
    type Err = TypesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.parse::<u64>().map_err(|e| {
            TypesError::invalid_string_as_sequence(s.to_string(), e)
        })?))
    }
}

impl Sequence {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn increment(&self) -> Sequence {
        Sequence(self.0 + 1)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for Sequence {
    fn from(seq: u64) -> Self {
        Sequence(seq)
    }
}

impl From<Sequence> for u64 {
    fn from(s: Sequence) -> u64 {
        s.0
    }
}

impl core::fmt::Debug for Sequence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        self.0.fmt(f)
    }
}

impl core::fmt::Display for Sequence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        self.0.fmt(f)
    }
}

impl core::ops::Add<Self> for Sequence {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Add<u64> for Sequence {
    type Output = Self;

    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}
