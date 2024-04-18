use std::{
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
};

use ibc_proto::ibc::core::channel::v1::{
    Channel as RawChannel, Counterparty as RawCounterparty,
    IdentifiedChannel as RawIdentifiedChannel,
};
use ibc_proto::Protobuf;
use serde::{Deserialize, Serialize};
use utils::pretty::PrettySlice;

use crate::ibc_core::ics24_host::identifier::{ChannelId, ConnectionId, PortId};

use super::{error::ChannelError, version::Version};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifiedChannelEnd {
    pub port_id: PortId,
    pub channel_id: ChannelId,
    pub channel_end: ChannelEnd,
}

impl IdentifiedChannelEnd {
    pub fn new(port_id: PortId, channel_id: ChannelId, channel_end: ChannelEnd) -> Self {
        IdentifiedChannelEnd {
            port_id,
            channel_id,
            channel_end,
        }
    }
}

impl Protobuf<RawIdentifiedChannel> for IdentifiedChannelEnd {}

impl TryFrom<RawIdentifiedChannel> for IdentifiedChannelEnd {
    type Error = ChannelError;

    fn try_from(value: RawIdentifiedChannel) -> Result<Self, Self::Error> {
        let raw_channel_end = RawChannel {
            state: value.state,
            ordering: value.ordering,
            counterparty: value.counterparty,
            connection_hops: value.connection_hops,
            version: value.version,
            upgrade_sequence: value.upgrade_sequence,
        };

        Ok(IdentifiedChannelEnd {
            port_id: value.port_id.parse().map_err(ChannelError::identifier)?,
            channel_id: value.channel_id.parse().map_err(ChannelError::identifier)?,
            channel_end: raw_channel_end.try_into()?,
        })
    }
}

impl From<IdentifiedChannelEnd> for RawIdentifiedChannel {
    fn from(value: IdentifiedChannelEnd) -> Self {
        RawIdentifiedChannel {
            state: value.channel_end.state as i32,
            ordering: value.channel_end.ordering as i32,
            counterparty: Some(value.channel_end.counterparty().clone().into()),
            connection_hops: value
                .channel_end
                .connection_hops
                .iter()
                .map(|v| v.as_str().to_string())
                .collect(),
            version: value.channel_end.version.to_string(),
            port_id: value.port_id.to_string(),
            channel_id: value.channel_id.to_string(),
            upgrade_sequence: value.channel_end.upgrade_sequence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelEnd {
    pub state: State,
    pub ordering: Ordering,
    pub remote: Counterparty,
    pub connection_hops: Vec<ConnectionId>,
    pub version: Version,
    pub upgrade_sequence: u64,
}

impl Display for ChannelEnd {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(
            f,
            "ChannelEnd {{ state: {}, ordering: {}, remote: {}, connection_hops: {}, version: {}, upgrade_sequence: {} }}",
            self.state, self.ordering, self.remote, PrettySlice(&self.connection_hops), self.version, self.upgrade_sequence
        )
    }
}

impl Default for ChannelEnd {
    fn default() -> Self {
        ChannelEnd {
            state: State::Uninitialized,
            ordering: Default::default(),
            remote: Counterparty::default(),
            connection_hops: Vec::new(),
            version: Version::default(),
            upgrade_sequence: 0,
        }
    }
}

impl Protobuf<RawChannel> for ChannelEnd {}

impl TryFrom<RawChannel> for ChannelEnd {
    type Error = ChannelError;

    fn try_from(value: RawChannel) -> Result<Self, Self::Error> {
        let chan_state: State = State::from_i32(value.state)?;

        if chan_state == State::Uninitialized {
            return Ok(ChannelEnd::default());
        }

        let chan_ordering = Ordering::from_i32(value.ordering)?;

        // Assemble the 'remote' attribute of the Channel, which represents the Counterparty.
        let remote = value
            .counterparty
            .ok_or_else(ChannelError::missing_counterparty)?
            .try_into()?;

        // Parse each item in connection_hops into a ConnectionId.
        let connection_hops = value
            .connection_hops
            .into_iter()
            .map(|conn_id| ConnectionId::from_str(conn_id.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ChannelError::identifier)?;

        let version = value.version.into();

        Ok(ChannelEnd::new(
            chan_state,
            chan_ordering,
            remote,
            connection_hops,
            version,
            value.upgrade_sequence,
        ))
    }
}

impl From<ChannelEnd> for RawChannel {
    fn from(value: ChannelEnd) -> Self {
        RawChannel {
            state: value.state as i32,
            ordering: value.ordering as i32,
            counterparty: Some(value.counterparty().clone().into()),
            connection_hops: value
                .connection_hops
                .iter()
                .map(|v| v.as_str().to_string())
                .collect(),
            version: value.version.to_string(),
            upgrade_sequence: value.upgrade_sequence,
        }
    }
}

impl ChannelEnd {
    /// Creates a new ChannelEnd in state Uninitialized and other fields parametrized.
    pub fn new(
        state: State,
        ordering: Ordering,
        remote: Counterparty,
        connection_hops: Vec<ConnectionId>,
        version: Version,
        upgrade_sequence: u64,
    ) -> Self {
        Self {
            state,
            ordering,
            remote,
            connection_hops,
            version,
            upgrade_sequence,
        }
    }

    /// Updates the ChannelEnd to assume a new State 's'.
    pub fn set_state(&mut self, s: State) {
        self.state = s;
    }

    pub fn set_version(&mut self, v: Version) {
        self.version = v;
    }

    pub fn set_counterparty_channel_id(&mut self, c: ChannelId) {
        self.remote.channel_id = Some(c);
    }

    /// Returns `true` if this `ChannelEnd` is in state [`State::Open`].
    pub fn is_open(&self) -> bool {
        self.state_matches(&State::Open)
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn ordering(&self) -> &Ordering {
        &self.ordering
    }

    pub fn counterparty(&self) -> &Counterparty {
        &self.remote
    }

    pub fn connection_hops(&self) -> &Vec<ConnectionId> {
        &self.connection_hops
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn validate_basic(&self) -> Result<(), ChannelError> {
        if self.connection_hops.len() != 1 {
            return Err(ChannelError::invalid_connection_hops_length(
                1,
                self.connection_hops.len(),
            ));
        }
        self.counterparty().validate_basic()
    }

    /// Helper function to compare the state of this end with another state.
    pub fn state_matches(&self, other: &State) -> bool {
        self.state.eq(other)
    }

    /// Helper function to compare the order of this end with another order.
    pub fn order_matches(&self, other: &Ordering) -> bool {
        self.ordering.eq(other)
    }

    #[allow(clippy::ptr_arg)]
    pub fn connection_hops_matches(&self, other: &Vec<ConnectionId>) -> bool {
        self.connection_hops.eq(other)
    }

    pub fn counterparty_matches(&self, other: &Counterparty) -> bool {
        self.counterparty().eq(other)
    }

    pub fn version_matches(&self, other: &Version) -> bool {
        self.version().eq(other)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counterparty {
    pub port_id: PortId,
    pub channel_id: Option<ChannelId>,
}

impl Counterparty {
    pub fn new(port_id: PortId, channel_id: Option<ChannelId>) -> Self {
        Self {
            port_id,
            channel_id,
        }
    }

    pub fn port_id(&self) -> &PortId {
        &self.port_id
    }

    pub fn channel_id(&self) -> Option<&ChannelId> {
        self.channel_id.as_ref()
    }

    pub fn validate_basic(&self) -> Result<(), ChannelError> {
        Ok(())
    }
}

impl Display for Counterparty {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match &self.channel_id {
            Some(channel_id) => write!(
                f,
                "Counterparty(port_id: {}, channel_id: {})",
                self.port_id, channel_id
            ),
            None => write!(
                f,
                "Counterparty(port_id: {}, channel_id: None)",
                self.port_id
            ),
        }
    }
}

impl Protobuf<RawCounterparty> for Counterparty {}

impl TryFrom<RawCounterparty> for Counterparty {
    type Error = ChannelError;

    fn try_from(value: RawCounterparty) -> Result<Self, Self::Error> {
        let channel_id = Some(value.channel_id)
            .filter(|x| !x.is_empty())
            .map(|v| FromStr::from_str(v.as_str()))
            .transpose()
            .map_err(ChannelError::identifier)?;
        Ok(Counterparty::new(
            value.port_id.parse().map_err(ChannelError::identifier)?,
            channel_id,
        ))
    }
}

impl From<Counterparty> for RawCounterparty {
    fn from(value: Counterparty) -> Self {
        RawCounterparty {
            port_id: value.port_id.as_str().to_string(),
            channel_id: value
                .channel_id
                .map_or_else(|| "".to_string(), |v| v.to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum Ordering {
    Uninitialized = 0,
    #[default]
    Unordered = 1,
    Ordered = 2,
}

impl Display for Ordering {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.as_str())
    }
}

impl Ordering {
    /// Yields the Order as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uninitialized => "UNINITIALIZED",
            Self::Unordered => "ORDER_UNORDERED",
            Self::Ordered => "ORDER_ORDERED",
        }
    }

    // Parses the Order out from a i32.
    pub fn from_i32(nr: i32) -> Result<Self, ChannelError> {
        match nr {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::Unordered),
            2 => Ok(Self::Ordered),

            _ => Err(ChannelError::unknown_order_type(nr.to_string())),
        }
    }
}

impl FromStr for Ordering {
    type Err = ChannelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim_start_matches("order_") {
            "uninitialized" => Ok(Self::Uninitialized),
            "unordered" => Ok(Self::Unordered),
            "ordered" => Ok(Self::Ordered),
            _ => Err(ChannelError::unknown_order_type(s.to_string())),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Uninitialized = 0,
    Init = 1,
    TryOpen = 2,
    Open = 3,
    Closed = 4,
}

impl State {
    /// Yields the state as a string
    pub fn as_string(&self) -> &'static str {
        match self {
            Self::Uninitialized => "UNINITIALIZED",
            Self::Init => "INIT",
            Self::TryOpen => "TRYOPEN",
            Self::Open => "OPEN",
            Self::Closed => "CLOSED",
        }
    }

    // Parses the State out from a i32.
    pub fn from_i32(s: i32) -> Result<Self, ChannelError> {
        match s {
            0 => Ok(Self::Uninitialized),
            1 => Ok(Self::Init),
            2 => Ok(Self::TryOpen),
            3 => Ok(Self::Open),
            4 => Ok(Self::Closed),
            _ => Err(ChannelError::unknown_state(s)),
        }
    }

    /// Returns whether or not this channel state is `Open`.
    pub fn is_open(self) -> bool {
        self == State::Open
    }

    /// Returns whether or not this channel state is `Closed`.
    pub fn is_closed(self) -> bool {
        self == State::Closed
    }

    /// Returns whether or not the channel with this state
    /// has progressed less or the same than the argument.
    ///
    /// # Example
    /// ```rust,ignore
    /// assert!(State::Init.less_or_equal_progress(State::Open));
    /// assert!(State::TryOpen.less_or_equal_progress(State::TryOpen));
    /// assert!(!State::Closed.less_or_equal_progress(State::Open));
    /// ```
    pub fn less_or_equal_progress(self, other: Self) -> bool {
        self as u32 <= other as u32
    }
}

/// Provides a `to_string` method.
impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.as_string())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChannelMsgType {
    OpenTry,
    OpenAck,
    OpenConfirm,
    CloseConfirm,
}

pub fn check_target_channel_state(
    channel_id: &ChannelId,
    existing_channel: &ChannelEnd,
    expected_channel: &ChannelEnd,
) -> Result<(), ChannelError> {
    let good_connection_hops =
        existing_channel.connection_hops() == expected_channel.connection_hops();

    let good_state = *existing_channel.state() as u32 <= *expected_channel.state() as u32;
    let good_channel_port_ids = existing_channel.counterparty().channel_id().is_none()
        || existing_channel.counterparty().channel_id()
            == expected_channel.counterparty().channel_id()
            && existing_channel.counterparty().port_id()
                == expected_channel.counterparty().port_id();


    if good_state && good_connection_hops && good_channel_port_ids {
        Ok(())
    } else {
        Err(ChannelError::channel_already_exist(channel_id.clone()))
    }
}