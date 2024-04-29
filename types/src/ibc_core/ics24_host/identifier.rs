use std::{
    convert::Infallible,
    fmt::{Display, Error, Formatter},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::error::TypesError;

use super::error::IdentifierError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "tendermint::chain::Id", into = "tendermint::chain::Id")]
pub struct ChainId {
    id: String,
    version: u64,
}

impl ChainId {
    /// Creates a new `ChainId` given a chain name and an epoch number.
    /// The returned `ChainId` will have the format: `{chain name}-{epoch number}`.
    pub fn new(name: String, version: u64) -> Self {
        Self {
            id: format!("{name}-{version}"),
            version,
        }
    }

    pub fn from_string(id: &str) -> Self {
        let version = if Self::is_epoch_format(id) {
            Self::chain_version(id)
        } else {
            0
        };

        Self {
            id: id.to_string(),
            version,
        }
    }

    /// Get a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.id
    }

    // TODO: this should probably be named epoch_number.
    /// Extract the version from this chain identifier.
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn chain_version(chain_id: &str) -> u64 {
        if !ChainId::is_epoch_format(chain_id) {
            return 0;
        }

        let split: Vec<_> = chain_id.split('-').collect();
        split
            .last()
            .expect("get revision number from chain_id")
            .parse()
            .unwrap_or(0)
    }

    /// is_epoch_format() checks if a chain_id is in the format required for parsing epochs
    /// The chainID must be in the form: `{chainID}-{version}`
    pub fn is_epoch_format(chain_id: &str) -> bool {
        let re = safe_regex::regex!(br".+[^-]-{1}[1-9][0-9]*");
        re.is_match(chain_id.as_bytes())
    }
}

impl FromStr for ChainId {
    type Err = Infallible;

    fn from_str(id: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_string(id))
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.id)
    }
}

impl From<ChainId> for tendermint::chain::Id {
    fn from(id: ChainId) -> Self {
        tendermint::chain::Id::from_str(id.as_str()).unwrap()
    }
}

impl From<tendermint::chain::Id> for ChainId {
    fn from(id: tendermint::chain::Id) -> Self {
        ChainId::from_str(id.as_str()).unwrap()
    }
}

impl Default for ChainId {
    fn default() -> Self {
        "defaultChainId".to_string().parse().unwrap()
    }
}

impl From<String> for ChainId {
    fn from(value: String) -> Self {
        Self::from_string(&value)
    }
}


/// assert_eq!(ChainId::chain_version("chain--a-0"), 0);
/// assert_eq!(ChainId::chain_version("ibc-10"), 10);
/// assert_eq!(ChainId::chain_version("cosmos-hub-97"), 97);
/// assert_eq!(ChainId::chain_version("testnet-helloworld-2"), 2);

pub fn chain_version(chain_id: &str) -> u64 {
    if !ChainId::is_epoch_format(chain_id) {
        return 0;
    }

    let split: Vec<_> = chain_id.split('-').collect();
    split
        .last()
        .expect("get revision number from chain_id")
        .parse()
        .unwrap_or(0)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientId(String);

impl ClientId {
    
    /// let tm_client_id = ClientId::new(ClientType::Tendermint, 0);
    /// assert!(tm_client_id.is_ok());
    /// tm_client_id.map(|id| { assert_eq!(&id, "07-tendermint-0") });
    pub fn new(client_type: &str, counter: u64) -> Result<Self, IdentifierError> {
        let id = format!("{client_type}-{counter}");
        Self::from_str(id.as_str())
    }

    /// Get this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get this identifier as a borrowed byte slice
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// This implementation provides a `to_string` method.
impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ClientId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_client_identifier(s).map(|_| Self(s.to_string()))
    }
}

impl Default for ClientId {
    fn default() -> Self {
        Self::new("07-tendermint", 0).unwrap()
    }
}


/// let client_id = ClientId::from_str("clientidtwo");
/// assert!(client_id.is_ok());
/// client_id.map(|id| {assert_eq!(&id, "clientidtwo")});

impl PartialEq<str> for ClientId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientType(String);

impl ClientType {
    /// Constructs a new `ClientType` from the given `String` if it ends with a valid client identifier.
    pub fn new(s: &str) -> Result<Self, IdentifierError> {
        let s_trim = s.trim();
        validate_client_type(s_trim)?;
        Ok(Self(s_trim.to_string()))
    }

    /// Yields this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ClientType {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "ClientType({})", self.0)
    }
}

impl Default for ClientType {
    fn default() -> Self {
        Self::new("07-tendermint").unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    const CHANNEL_PREFIX: &'static str = "channel-";

 
    /// ```
    /// # use ibc_relayer_types::core::ics24_host::identifier::ChannelId;
    /// let chan_id = ChannelId::new(27);
    /// assert_eq!(chan_id.to_string(), "channel-27");
    /// ```
    pub fn new(counter: u64) -> Self {
        let id = format!("{}{}", Self::CHANNEL_PREFIX, counter);
        Self(id)
    }

    /// Get this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get this identifier as a borrowed byte slice
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// This implementation provides a `to_string` method.
impl Display for ChannelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ChannelId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_channel_identifier(s).map(|_| Self(s.to_string()))
    }
}

impl AsRef<str> for ChannelId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for ChannelId {
    fn default() -> Self {
        Self::new(0)
    }
}

/// ```
/// let channel_id = ChannelId::from_str("channelId-0");
/// assert!(channel_id.is_ok());
/// channel_id.map(|id| {assert_eq!(&id, "channelId-0")});
/// ```
impl PartialEq<str> for ChannelId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

/// Path separator (ie. forward slash '/')
const PATH_SEPARATOR: char = '/';
const VALID_SPECIAL_CHARS: &str = "._+-#[]<>";

/// Default validator function for identifiers.
///
/// A valid identifier only contain lowercase alphabetic characters, and be of a given min and max
/// length.
pub fn validate_identifier(id: &str, min: usize, max: usize) -> Result<(), IdentifierError> {
    assert!(max >= min);

    // Check identifier is not empty
    if id.is_empty() {
        return Err(IdentifierError::id_empty());
    }

    // Check identifier does not contain path separators
    if id.contains(PATH_SEPARATOR) {
        return Err(IdentifierError::id_contain_separator(id.to_string()));
    }

    // Check identifier length is between given min/max
    if id.len() < min || id.len() > max {
        return Err(IdentifierError::id_invalid_length(
            id.to_string(),
            id.len(),
            min,
            max,
        ));
    }

    // Check that the identifier comprises only valid characters:
    // - Alphanumeric
    // - `.`, `_`, `+`, `-`, `#`
    // - `[`, `]`, `<`, `>`
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || VALID_SPECIAL_CHARS.contains(c))
    {
        return Err(IdentifierError::id_invalid_character(id.to_string()));
    }

    // All good!
    Ok(())
}

/// Default validator function for Client identifiers.
///
/// A valid identifier must be between 9-64 characters and only contain lowercase
/// alphabetic characters,
pub fn validate_client_identifier(id: &str) -> Result<(), IdentifierError> {
    validate_identifier(id, 9, 64)
}

pub fn validate_client_type(id: &str) -> Result<(), IdentifierError> {
    validate_identifier(id, 9, 64)
}

/// Default validator function for Connection identifiers.
///
/// A valid Identifier must be between 10-64 characters and only contain lowercase
/// alphabetic characters,
pub fn validate_connection_identifier(id: &str) -> Result<(), IdentifierError> {
    validate_identifier(id, 10, 64)
}

/// Default validator function for Port identifiers.
///
/// A valid Identifier must be between 2-128 characters and only contain lowercase
/// alphabetic characters,
pub fn validate_port_identifier(id: &str) -> Result<(), IdentifierError> {
    validate_identifier(id, 2, 128)
}

/// Default validator function for Channel identifiers.
///
/// A valid identifier must be between 8-64 characters and only contain
/// alphabetic characters,
pub fn validate_channel_identifier(id: &str) -> Result<(), IdentifierError> {
    validate_identifier(id, 8, 64)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PortId(String);

impl PortId {
    /// Infallible creation of the well-known transfer port
    pub fn transfer() -> Self {
        Self("transfer".to_string())
    }

    pub fn oracle() -> Self {
        Self("oracle".to_string())
    }

    pub fn icqhost() -> Self {
        Self("icqhost".to_string())
    }

    /// Get this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get this identifier as a borrowed byte slice
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// This implementation provides a `to_string` method.
impl Display for PortId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PortId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_port_identifier(s).map(|_| Self(s.to_string()))
        // Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for PortId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for PortId {
    fn default() -> Self {
        "defaultPort".to_string().parse().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConnectionId(String);

impl ConnectionId {
    /// Builds a new connection identifier. Connection identifiers are deterministically formed from
    /// two elements: a prefix `prefix`, and a monotonically increasing `counter`; these are
    /// separated by a dash "-". The prefix is currently determined statically (see
    /// `ConnectionId::prefix()`) so this method accepts a single argument, the `counter`.
   
    pub fn new(counter: u64) -> Self {
        let id = format!("{}-{}", Self::prefix(), counter);
        Self::from_str(id.as_str()).unwrap()
    }

    /// Returns the static prefix to be used across all connection identifiers.
    pub fn prefix() -> &'static str {
        "connection"
    }

    /// Get this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get this identifier as a borrowed byte slice
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// This implementation provides a `to_string` method.
impl Display for ConnectionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ConnectionId {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_connection_identifier(s).map(|_| Self(s.to_string()))
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Equality check against string literal (satisfies &ConnectionId == &str).
/// ```
/// let conn_id = ConnectionId::from_str("connectionId-0");
/// assert!(conn_id.is_ok());
/// conn_id.map(|id| {assert_eq!(&id, "connectionId-0")});
/// ```
impl PartialEq<str> for ConnectionId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
