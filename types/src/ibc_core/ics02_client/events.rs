use std::fmt::{Display, Formatter, Error};

use serde::{Deserialize, Serialize};
use tendermint::abci;
use utils::encode::systems::hex_encode;

use crate::{ibc_core::ics24_host::identifier::{ClientId, ClientType}, light_clients::ics07_tendermint::height::Height, ibc_events::{IbcEvent, IbcEventType}};

/// The content of the `key` field for the attribute containing the client identifier.
pub const CLIENT_ID_ATTRIBUTE_KEY: &str = "client_id";
/// The content of the `key` field for the attribute containing the client type.
pub const CLIENT_TYPE_ATTRIBUTE_KEY: &str = "client_type";
/// The content of the `key` field for the attribute containing the height.
pub const CONSENSUS_HEIGHT_ATTRIBUTE_KEY: &str = "consensus_height";
/// The content of the `key` field for the header in update client event.
pub const HEADER_ATTRIBUTE_KEY: &str = "header";

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
pub struct CreateClientEvent(pub Attributes);

impl CreateClientEvent {
    pub fn client_id(&self) -> &ClientId {
        &self.0.client_id
    }
}

impl Display for CreateClientEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "CreateClient {{ {} }}", self.0)
    }
}

impl From<Attributes> for CreateClientEvent {
    fn from(attrs: Attributes) -> Self {
        CreateClientEvent(attrs)
    }
}

impl From<CreateClientEvent> for IbcEvent {
    fn from(v: CreateClientEvent) -> Self {
        IbcEvent::CreateClient(v)
    }
}

impl From<CreateClientEvent> for abci::Event {
    fn from(v: CreateClientEvent) -> Self {
        Self {
            kind: IbcEventType::CreateClient.as_str().to_owned(),
            attributes: v.0.into(),
        }
    }
}

/// UpdateClient event signals a recent update of an on-chain client (IBC Client).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UpdateClientEvent {
    pub common: Attributes,
    pub header: Option<Vec<u8>>,
}

impl UpdateClientEvent {
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

impl Display for UpdateClientEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // TODO Display: Check for a solution for Box<dyn Header>
        write!(
            f,
            "UpdateClientEvent {{ common: {}, header: None }}",
            self.common
        )
    }
}

impl From<Attributes> for UpdateClientEvent {
    fn from(attrs: Attributes) -> Self {
        UpdateClientEvent {
            common: attrs,
            header: None,
        }
    }
}

impl From<UpdateClientEvent> for IbcEvent {
    fn from(v: UpdateClientEvent) -> Self {
        IbcEvent::UpdateClient(v)
    }
}

impl From<UpdateClientEvent> for abci::Event {
    fn from(v: UpdateClientEvent) -> Self {
        let mut attributes: Vec<_> = v.common.into();
        if let Some(h) = v.header {
            let h = String::from_utf8(hex_encode(h)).expect("hex-encoded string should always be valid UTF-8");
            let header = (HEADER_ATTRIBUTE_KEY, h).into();
            attributes.push(header);
        }
        Self {
            kind: IbcEventType::UpdateClient.as_str().to_string(),
            attributes,
        }
    }
}