use crate::ibc_core::ics24_host::identifier::ChannelId;

use super::timeout::TimeoutHeight;

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
    /// Checks whether a packet from a
    /// [`SendPacket`](crate::core::ics04_channel::events::SendPacket)
    /// event is timed-out relative to the current state of the
    /// destination chain.
    ///
    /// Checks both for time-out relative to the destination chain's
    /// current timestamp `dst_chain_ts` as well as relative to
    /// the height `dst_chain_height`.
    ///
    /// Note: a timed-out packet should result in a
    /// [`MsgTimeout`](crate::core::ics04_channel::msgs::timeout::MsgTimeout),
    /// instead of the common-case where it results in
    /// [`MsgRecvPacket`](crate::core::ics04_channel::msgs::recv_packet::MsgRecvPacket).
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
