use std::fmt::{Display, Formatter, Error};

use ibc_proto::Protobuf;
use serde::{Deserialize, Serialize};
use tendermint::abci;
use utils::encode::systems::hex_encode;

use crate::{ibc_core::ics24_host::identifier::{ClientId, ClientType}, ibc_events::{IbcEvent, IbcEventType}};

use super::{header::AnyHeader, height::Height};

/// The content of the `key` field for the attribute containing the client identifier.
pub const CLIENT_ID_ATTRIBUTE_KEY: &str = "client_id";
/// The content of the `key` field for the attribute containing the client type.
pub const CLIENT_TYPE_ATTRIBUTE_KEY: &str = "client_type";
/// The content of the `key` field for the attribute containing the height.
pub const CONSENSUS_HEIGHT_ATTRIBUTE_KEY: &str = "consensus_height";
/// The content of the `key` field for the header in update client event.
pub const HEADER_ATTRIBUTE_KEY: &str = "header";

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct NewBlock {
    pub height: Height,
}

impl NewBlock {
    pub fn new(h: Height) -> NewBlock {
        NewBlock { height: h }
    }
    pub fn set_height(&mut self, height: Height) {
        self.height = height;
    }
    pub fn height(&self) -> Height {
        self.height
    }
}

impl Display for NewBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "NewBlock {{ height: {} }}", self.height)
    }
}

impl From<NewBlock> for IbcEvent {
    fn from(v: NewBlock) -> Self {
        IbcEvent::NewBlock(v)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Attributes {
    pub client_id: ClientId,
    pub client_type: ClientType,
    pub consensus_height: Height,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            client_id: Default::default(),
            client_type: Default::default(),
            consensus_height: Height::new(0, 1).unwrap(),
        }
    }
}

impl Display for Attributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Attributes {{ client_id: {}, client_type: {}, consensus_height: {} }}",
            self.client_id, self.client_type, self.consensus_height
        )
    }
}

/// Convert attributes to Tendermint ABCI tags
impl From<Attributes> for Vec<abci::EventAttribute> {
    fn from(attrs: Attributes) -> Self {
        let client_id = (CLIENT_ID_ATTRIBUTE_KEY, attrs.client_id.as_str()).into();
        let client_type = (CLIENT_TYPE_ATTRIBUTE_KEY, attrs.client_type.as_str()).into();
        let consensus_height = (
            CONSENSUS_HEIGHT_ATTRIBUTE_KEY,
            attrs.consensus_height.to_string(),
        )
            .into();
        vec![client_id, client_type, consensus_height]
    }
}



/// CreateClient event signals the creation of a new on-chain client (IBC client).
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct CreateClient(pub Attributes);

impl CreateClient {
    pub fn client_id(&self) -> &ClientId {
        &self.0.client_id
    }
}

impl Display for CreateClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "CreateClient {{ {} }}", self.0)
    }
}

impl From<Attributes> for CreateClient {
    fn from(attrs: Attributes) -> Self {
        CreateClient(attrs)
    }
}

impl From<CreateClient> for IbcEvent {
    fn from(v: CreateClient) -> Self {
        IbcEvent::CreateClient(v)
    }
}

impl From<CreateClient> for abci::Event {
    fn from(v: CreateClient) -> Self {
        Self {
            kind: IbcEventType::CreateClient.as_str().to_owned(),
            attributes: v.0.into(),
        }
    }
}

/// UpdateClient event signals a recent update of an on-chain client (IBC Client).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UpdateClient {
    pub common: Attributes,
    pub header: Option<AnyHeader>,
}

impl UpdateClient {
    pub fn client_id(&self) -> &ClientId {
        &self.common.client_id
    }

    pub fn client_type(&self) -> ClientType {
        self.common.client_type.clone()
    }

    pub fn consensus_height(&self) -> Height {
        self.common.consensus_height
    }
}

impl Display for UpdateClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "UpdateClient {{ {} }}", self.common)
    }
}

impl From<Attributes> for UpdateClient {
    fn from(attrs: Attributes) -> Self {
        UpdateClient {
            common: attrs,
            header: None,
        }
    }
}

impl From<UpdateClient> for IbcEvent {
    fn from(v: UpdateClient) -> Self {
        IbcEvent::UpdateClient(v)
    }
}

fn encode_to_hex_string(header: AnyHeader) -> String {
    let buf = header.encode_vec();
    let encoded = subtle_encoding::hex::encode(buf);
    String::from_utf8(encoded).expect("hex-encoded string should always be valid UTF-8")
}

impl From<UpdateClient> for abci::Event {
    fn from(v: UpdateClient) -> Self {
        let mut attributes: Vec<_> = v.common.into();

        if let Some(h) = v.header {
            let header = (HEADER_ATTRIBUTE_KEY, encode_to_hex_string(h)).into();
            attributes.push(header);
        }

        Self {
            kind: IbcEventType::UpdateClient.as_str().to_string(),
            attributes,
        }
    }
}

/// Signals a recent upgrade of an on-chain client (IBC Client).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct UpgradeClient(pub Attributes);

impl UpgradeClient {
    pub fn client_id(&self) -> &ClientId {
        &self.0.client_id
    }
}

impl Display for UpgradeClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "UpgradeClient {{ {} }}", self.0)
    }
}

impl From<Attributes> for UpgradeClient {
    fn from(attrs: Attributes) -> Self {
        UpgradeClient(attrs)
    }
}

impl From<UpgradeClient> for abci::Event {
    fn from(v: UpgradeClient) -> Self {
        Self {
            kind: IbcEventType::UpgradeClient.as_str().to_owned(),
            attributes: v.0.into(),
        }
    }
}
