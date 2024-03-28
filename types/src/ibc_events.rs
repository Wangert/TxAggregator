use crate::error::TypesError;
use crate::ibc_core::ics02_client::events::{
    self as ClientEvents, Attributes as ClientAttributes, NewBlock, HEADER_ATTRIBUTE_KEY,
};
use crate::ibc_core::ics02_client::header::{decode_header, AnyHeader};
use crate::ibc_core::ics02_client::height::Height;
use crate::ibc_core::ics03_connection::events::{
    self as ConnectionEvents, Attributes as ConnectionAttributes,
};
use crate::ibc_core::ics04_channel::events::{
    self as ChannelEvents, Attributes as ChannelAttributes,
};
use crate::ibc_core::ics04_channel::packet::Packet;
use flex_error::{define_error, TraceError};
use ibc_proto::google::protobuf::field_descriptor_proto::Type;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::{Display, Error as FmtError, Formatter};
use std::str::FromStr;
use subtle_encoding::hex;
use tendermint::abci::{self, Event as AbciEvent, EventAttribute};
use tendermint_proto::serializers::bytes::base64string;
use utils::encode::base64;

/// Events whose data is not included in the app state and must be extracted using tendermint RPCs
/// (i.e. /tx_search or /block_search)
// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub enum WithBlockDataType {
//     CreateClient,
//     UpdateClient,
//     SendPacket,
//     WriteAck,
// }

// impl WithBlockDataType {
//     pub fn as_str(&self) -> &'static str {
//         match *self {
//             WithBlockDataType::CreateClient => "create_client",
//             WithBlockDataType::UpdateClient => "update_client",
//             WithBlockDataType::SendPacket => "send_packet",
//             WithBlockDataType::WriteAck => "write_acknowledgement",
//         }
//     }
// }

const NEW_BLOCK_EVENT: &str = "new_block";
const EMPTY_EVENT: &str = "empty";
const CHAIN_ERROR_EVENT: &str = "chain_error";
const APP_MODULE_EVENT: &str = "app_module";
/// Client event types
const CREATE_CLIENT_EVENT: &str = "create_client";
const UPDATE_CLIENT_EVENT: &str = "update_client";
const CLIENT_MISBEHAVIOUR_EVENT: &str = "client_misbehaviour";
const UPGRADE_CLIENT_EVENT: &str = "upgrade_client";
/// Connection event types
const CONNECTION_INIT_EVENT: &str = "connection_open_init";
const CONNECTION_TRY_EVENT: &str = "connection_open_try";
const CONNECTION_ACK_EVENT: &str = "connection_open_ack";
const CONNECTION_CONFIRM_EVENT: &str = "connection_open_confirm";
/// Channel event types
const CHANNEL_OPEN_INIT_EVENT: &str = "channel_open_init";
const CHANNEL_OPEN_TRY_EVENT: &str = "channel_open_try";
const CHANNEL_OPEN_ACK_EVENT: &str = "channel_open_ack";
const CHANNEL_OPEN_CONFIRM_EVENT: &str = "channel_open_confirm";
const CHANNEL_CLOSE_INIT_EVENT: &str = "channel_close_init";
const CHANNEL_CLOSE_CONFIRM_EVENT: &str = "channel_close_confirm";
/// Packet event types
const SEND_PACKET_EVENT: &str = "send_packet";
const RECEIVE_PACKET_EVENT: &str = "receive_packet";
const WRITE_ACK_EVENT: &str = "write_acknowledgement";
const ACK_PACKET_EVENT: &str = "acknowledge_packet";
const TIMEOUT_EVENT: &str = "timeout_packet";
const TIMEOUT_ON_CLOSE_EVENT: &str = "timeout_packet_on_close";
const INCENTIVIZED_PACKET_EVENT: &str = "incentivized_ibc_packet";
/// CrossChainQuery event type
const CROSS_CHAIN_QUERY_PACKET_EVENT: &str = "cross_chain_query";
/// Distribution fee event type
const DISTRIBUTION_FEE_PACKET_EVENT: &str = "distribute_fee";

#[derive(Clone, Debug, Serialize)]
pub struct IbcEventWithHeight {
    pub event: IbcEvent,
    pub height: Height,
}

impl IbcEventWithHeight {
    pub fn new(event: IbcEvent, height: Height) -> Self {
        Self { event, height }
    }

    pub fn with_height(self, height: Height) -> Self {
        Self {
            event: self.event,
            height,
        }
    }
}

impl Display for IbcEventWithHeight {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "{} at height {}", self.event, self.height)
    }
}

/// Events types
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum IbcEventType {
    NewBlock,
    CreateClient,
    UpdateClient,
    UpgradeClient,
    ClientMisbehaviour,
    OpenInitConnection,
    OpenTryConnection,
    OpenAckConnection,
    OpenConfirmConnection,
    OpenInitChannel,
    OpenTryChannel,
    OpenAckChannel,
    OpenConfirmChannel,
    CloseInitChannel,
    CloseConfirmChannel,
    SendPacket,
    ReceivePacket,
    WriteAck,
    AckPacket,
    Timeout,
    TimeoutOnClose,
    IncentivizedPacket,
    CrossChainQuery,
    AppModule,
    Empty,
    ChainError,
    DistributionFee,
}

impl IbcEventType {
    pub fn as_str(&self) -> &'static str {
        match *self {
            IbcEventType::NewBlock => NEW_BLOCK_EVENT,
            IbcEventType::CreateClient => CREATE_CLIENT_EVENT,
            IbcEventType::UpdateClient => UPDATE_CLIENT_EVENT,
            IbcEventType::UpgradeClient => UPGRADE_CLIENT_EVENT,
            IbcEventType::ClientMisbehaviour => CLIENT_MISBEHAVIOUR_EVENT,
            IbcEventType::OpenInitConnection => CONNECTION_INIT_EVENT,
            IbcEventType::OpenTryConnection => CONNECTION_TRY_EVENT,
            IbcEventType::OpenAckConnection => CONNECTION_ACK_EVENT,
            IbcEventType::OpenConfirmConnection => CONNECTION_CONFIRM_EVENT,
            IbcEventType::OpenInitChannel => CHANNEL_OPEN_INIT_EVENT,
            IbcEventType::OpenTryChannel => CHANNEL_OPEN_TRY_EVENT,
            IbcEventType::OpenAckChannel => CHANNEL_OPEN_ACK_EVENT,
            IbcEventType::OpenConfirmChannel => CHANNEL_OPEN_CONFIRM_EVENT,
            IbcEventType::CloseInitChannel => CHANNEL_CLOSE_INIT_EVENT,
            IbcEventType::CloseConfirmChannel => CHANNEL_CLOSE_CONFIRM_EVENT,
            IbcEventType::SendPacket => SEND_PACKET_EVENT,
            IbcEventType::ReceivePacket => RECEIVE_PACKET_EVENT,
            IbcEventType::WriteAck => WRITE_ACK_EVENT,
            IbcEventType::AckPacket => ACK_PACKET_EVENT,
            IbcEventType::Timeout => TIMEOUT_EVENT,
            IbcEventType::TimeoutOnClose => TIMEOUT_ON_CLOSE_EVENT,
            IbcEventType::IncentivizedPacket => INCENTIVIZED_PACKET_EVENT,
            IbcEventType::CrossChainQuery => CROSS_CHAIN_QUERY_PACKET_EVENT,
            IbcEventType::AppModule => APP_MODULE_EVENT,
            IbcEventType::Empty => EMPTY_EVENT,
            IbcEventType::ChainError => CHAIN_ERROR_EVENT,
            IbcEventType::DistributionFee => DISTRIBUTION_FEE_PACKET_EVENT,
        }
    }
}

impl FromStr for IbcEventType {
    type Err = TypesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            NEW_BLOCK_EVENT => Ok(IbcEventType::NewBlock),
            CREATE_CLIENT_EVENT => Ok(IbcEventType::CreateClient),
            UPDATE_CLIENT_EVENT => Ok(IbcEventType::UpdateClient),
            UPGRADE_CLIENT_EVENT => Ok(IbcEventType::UpgradeClient),
            CLIENT_MISBEHAVIOUR_EVENT => Ok(IbcEventType::ClientMisbehaviour),
            CONNECTION_INIT_EVENT => Ok(IbcEventType::OpenInitConnection),
            CONNECTION_TRY_EVENT => Ok(IbcEventType::OpenTryConnection),
            CONNECTION_ACK_EVENT => Ok(IbcEventType::OpenAckConnection),
            CONNECTION_CONFIRM_EVENT => Ok(IbcEventType::OpenConfirmConnection),
            CHANNEL_OPEN_INIT_EVENT => Ok(IbcEventType::OpenInitChannel),
            CHANNEL_OPEN_TRY_EVENT => Ok(IbcEventType::OpenTryChannel),
            CHANNEL_OPEN_ACK_EVENT => Ok(IbcEventType::OpenAckChannel),
            CHANNEL_OPEN_CONFIRM_EVENT => Ok(IbcEventType::OpenConfirmChannel),
            CHANNEL_CLOSE_INIT_EVENT => Ok(IbcEventType::CloseInitChannel),
            CHANNEL_CLOSE_CONFIRM_EVENT => Ok(IbcEventType::CloseConfirmChannel),
            SEND_PACKET_EVENT => Ok(IbcEventType::SendPacket),
            RECEIVE_PACKET_EVENT => Ok(IbcEventType::ReceivePacket),
            WRITE_ACK_EVENT => Ok(IbcEventType::WriteAck),
            ACK_PACKET_EVENT => Ok(IbcEventType::AckPacket),
            TIMEOUT_EVENT => Ok(IbcEventType::Timeout),
            TIMEOUT_ON_CLOSE_EVENT => Ok(IbcEventType::TimeoutOnClose),
            INCENTIVIZED_PACKET_EVENT => Ok(IbcEventType::IncentivizedPacket),
            CROSS_CHAIN_QUERY_PACKET_EVENT => Ok(IbcEventType::CrossChainQuery),
            EMPTY_EVENT => Ok(IbcEventType::Empty),
            CHAIN_ERROR_EVENT => Ok(IbcEventType::ChainError),
            DISTRIBUTION_FEE_PACKET_EVENT => Ok(IbcEventType::DistributionFee),
            // from_str() for `APP_MODULE_EVENT` MUST fail because a `ModuleEvent`'s type isn't constant
            _ => Err(TypesError::incorrect_event_type(s.to_string())),
        }
    }
}

/// Events created by the IBC component of a chain, destined for a relayer.
#[derive(Debug, Clone, Serialize)]
pub enum IbcEvent {
    NewBlock(NewBlock),
    CreateClient(ClientEvents::CreateClient),
    UpdateClient(ClientEvents::UpdateClient),
    UpgradeClient(ClientEvents::UpgradeClient),
    // ClientMisbehaviour(ClientEvents::ClientMisbehaviour),
    OpenInitConnection(ConnectionEvents::OpenInit),
    OpenTryConnection(ConnectionEvents::OpenTry),
    OpenAckConnection(ConnectionEvents::OpenAck),
    OpenConfirmConnection(ConnectionEvents::OpenConfirm),

    OpenInitChannel(ChannelEvents::OpenInit),
    OpenTryChannel(ChannelEvents::OpenTry),
    OpenAckChannel(ChannelEvents::OpenAck),
    OpenConfirmChannel(ChannelEvents::OpenConfirm),
    CloseInitChannel(ChannelEvents::CloseInit),
    CloseConfirmChannel(ChannelEvents::CloseConfirm),

    SendPacket(ChannelEvents::SendPacket),
    ReceivePacket(ChannelEvents::ReceivePacket),
    WriteAcknowledgement(ChannelEvents::WriteAcknowledgement),
    AcknowledgePacket(ChannelEvents::AcknowledgePacket),
    TimeoutPacket(ChannelEvents::TimeoutPacket),
    TimeoutOnClosePacket(ChannelEvents::TimeoutOnClosePacket),

    // IncentivizedPacket(IncentivizedPacket),
    // CrossChainQueryPacket(CrossChainQueryPacket),

    // DistributeFeePacket(DistributeFeePacket),

    // AppModule(ModuleEvent),
    CosmosChainError(String), // Special event, signifying an error on CheckTx or DeliverTx
}

impl Display for IbcEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            IbcEvent::NewBlock(ev) => write!(f, "NewBlock({})", ev.height),
            IbcEvent::CreateClient(ev) => write!(f, "CreateClient({ev})"),
            IbcEvent::UpdateClient(ev) => write!(f, "UpdateClient({ev})"),
            IbcEvent::UpgradeClient(ev) => write!(f, "UpgradeClient({ev})"),
            // IbcEvent::ClientMisbehaviour(ev) => write!(f, "ClientMisbehaviour({ev})"),
            IbcEvent::OpenInitConnection(ev) => write!(f, "OpenInitConnection({ev})"),
            IbcEvent::OpenTryConnection(ev) => write!(f, "OpenTryConnection({ev})"),
            IbcEvent::OpenAckConnection(ev) => write!(f, "OpenAckConnection({ev})"),
            IbcEvent::OpenConfirmConnection(ev) => write!(f, "OpenConfirmConnection({ev})"),

            IbcEvent::OpenInitChannel(ev) => write!(f, "OpenInitChannel({ev})"),
            IbcEvent::OpenTryChannel(ev) => write!(f, "OpenTryChannel({ev})"),
            IbcEvent::OpenAckChannel(ev) => write!(f, "OpenAckChannel({ev})"),
            IbcEvent::OpenConfirmChannel(ev) => write!(f, "OpenConfirmChannel({ev})"),
            IbcEvent::CloseInitChannel(ev) => write!(f, "CloseInitChannel({ev})"),
            IbcEvent::CloseConfirmChannel(ev) => write!(f, "CloseConfirmChannel({ev})"),

            IbcEvent::SendPacket(ev) => write!(f, "SendPacket({ev})"),
            IbcEvent::ReceivePacket(ev) => write!(f, "ReceivePacket({ev})"),
            IbcEvent::WriteAcknowledgement(ev) => write!(f, "WriteAcknowledgement({ev})"),
            IbcEvent::AcknowledgePacket(ev) => write!(f, "AcknowledgePacket({ev})"),
            IbcEvent::TimeoutPacket(ev) => write!(f, "TimeoutPacket({ev})"),
            IbcEvent::TimeoutOnClosePacket(ev) => write!(f, "TimeoutOnClosePacket({ev})"),

            // IbcEvent::IncentivizedPacket(ev) => write!(f, "IncenvitizedPacket({ev:?}"),
            // IbcEvent::CrossChainQueryPacket(ev) => write!(f, "CrosschainPacket({ev:?})"),

            // IbcEvent::DistributeFeePacket(ev) => write!(f, "DistributionFeePacket({ev:?})"),

            // IbcEvent::AppModule(ev) => write!(f, "AppModule({ev})"),
            IbcEvent::CosmosChainError(ev) => write!(f, "ChainError({ev})"),
        }
    }
}

// impl TryFrom<IbcEvent> for abci::Event {
//     type Error = TypesError;

//     fn try_from(event: IbcEvent) -> Result<Self, Self::Error> {
//         Ok(match event {
//             IbcEvent::NewBlock(event) => event.into(),
//             IbcEvent::CreateClient(event) => event.into(),
//             IbcEvent::UpdateClient(event) => event.into(),
//             // IbcEvent::UpgradeClient(event) => event.into(),
//             // IbcEvent::ClientMisbehaviour(event) => event.into(),
//             // IbcEvent::OpenInitConnection(event) => event.into(),
//             // IbcEvent::OpenTryConnection(event) => event.into(),
//             // IbcEvent::OpenAckConnection(event) => event.into(),
//             // IbcEvent::OpenConfirmConnection(event) => event.into(),
//             // IbcEvent::OpenInitChannel(event) => event.into(),
//             // IbcEvent::OpenTryChannel(event) => event.into(),
//             // IbcEvent::OpenAckChannel(event) => event.into(),
//             // IbcEvent::OpenConfirmChannel(event) => event.into(),
//             // IbcEvent::CloseInitChannel(event) => event.into(),
//             // IbcEvent::CloseConfirmChannel(event) => event.into(),
//             // IbcEvent::SendPacket(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::ReceivePacket(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::WriteAcknowledgement(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::AcknowledgePacket(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::TimeoutPacket(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::TimeoutOnClosePacket(event) => event.try_into().map_err(Error::channel)?,
//             // IbcEvent::IncentivizedPacket(event) => event.into(),
//             // IbcEvent::CrossChainQueryPacket(event) => event.into(),
//             // IbcEvent::DistributeFeePacket(event) => event.into(),
//             // IbcEvent::AppModule(event) => event.try_into()?,
//             IbcEvent::CosmosChainError(_) => {
//                 return Err(TypesError::incorrect_event_type(event.to_string()));
//             }
//         })
//     }
// }

impl IbcEvent {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(value) => value,
            Err(_) => format!("{self:?}"), // Fallback to debug printing
        }
    }

    pub fn event_type(&self) -> IbcEventType {
        match self {
            IbcEvent::NewBlock(_) => IbcEventType::NewBlock,
            IbcEvent::CreateClient(_) => IbcEventType::CreateClient,
            IbcEvent::UpdateClient(_) => IbcEventType::UpdateClient,
            // IbcEvent::ClientMisbehaviour(_) => IbcEventType::ClientMisbehaviour,
            IbcEvent::UpgradeClient(_) => IbcEventType::UpgradeClient,
            IbcEvent::OpenInitConnection(_) => IbcEventType::OpenInitConnection,
            IbcEvent::OpenTryConnection(_) => IbcEventType::OpenTryConnection,
            IbcEvent::OpenAckConnection(_) => IbcEventType::OpenAckConnection,
            IbcEvent::OpenConfirmConnection(_) => IbcEventType::OpenConfirmConnection,
            IbcEvent::OpenInitChannel(_) => IbcEventType::OpenInitChannel,
            IbcEvent::OpenTryChannel(_) => IbcEventType::OpenTryChannel,
            IbcEvent::OpenAckChannel(_) => IbcEventType::OpenAckChannel,
            IbcEvent::OpenConfirmChannel(_) => IbcEventType::OpenConfirmChannel,
            IbcEvent::CloseInitChannel(_) => IbcEventType::CloseInitChannel,
            IbcEvent::CloseConfirmChannel(_) => IbcEventType::CloseConfirmChannel,
            IbcEvent::SendPacket(_) => IbcEventType::SendPacket,
            IbcEvent::ReceivePacket(_) => IbcEventType::ReceivePacket,
            IbcEvent::WriteAcknowledgement(_) => IbcEventType::WriteAck,
            IbcEvent::AcknowledgePacket(_) => IbcEventType::AckPacket,
            IbcEvent::TimeoutPacket(_) => IbcEventType::Timeout,
            IbcEvent::TimeoutOnClosePacket(_) => IbcEventType::TimeoutOnClose,
            // IbcEvent::IncentivizedPacket(_) => IbcEventType::IncentivizedPacket,
            // IbcEvent::CrossChainQueryPacket(_) => IbcEventType::CrossChainQuery,
            // IbcEvent::DistributeFeePacket(_) => IbcEventType::DistributionFee,
            // IbcEvent::AppModule(_) => IbcEventType::AppModule,
            IbcEvent::CosmosChainError(_) => IbcEventType::ChainError,
        }
    }

    pub fn channel_attributes(self) -> Option<ChannelAttributes> {
        match self {
            IbcEvent::OpenInitChannel(ev) => Some(ev.into()),
            IbcEvent::OpenTryChannel(ev) => Some(ev.into()),
            IbcEvent::OpenAckChannel(ev) => Some(ev.into()),
            IbcEvent::OpenConfirmChannel(ev) => Some(ev.into()),
            _ => None,
        }
    }

    pub fn connection_attributes(&self) -> Option<&ConnectionAttributes> {
        match self {
            IbcEvent::OpenInitConnection(ev) => Some(ev.attributes()),
            IbcEvent::OpenTryConnection(ev) => Some(ev.attributes()),
            IbcEvent::OpenAckConnection(ev) => Some(ev.attributes()),
            IbcEvent::OpenConfirmConnection(ev) => Some(ev.attributes()),
            _ => None,
        }
    }

    pub fn packet(&self) -> Option<&Packet> {
        match self {
            IbcEvent::SendPacket(ev) => Some(&ev.packet),
            IbcEvent::ReceivePacket(ev) => Some(&ev.packet),
            IbcEvent::WriteAcknowledgement(ev) => Some(&ev.packet),
            IbcEvent::AcknowledgePacket(ev) => Some(&ev.packet),
            IbcEvent::TimeoutPacket(ev) => Some(&ev.packet),
            IbcEvent::TimeoutOnClosePacket(ev) => Some(&ev.packet),
            _ => None,
        }
    }

    // pub fn cross_chain_query_packet(&self) -> Option<&CrossChainQueryPacket> {
    //     match self {
    //         IbcEvent::CrossChainQueryPacket(ev) => Some(ev),
    //         _ => None,
    //     }
    // }

    pub fn ack(&self) -> Option<&[u8]> {
        match self {
            IbcEvent::WriteAcknowledgement(ev) => Some(&ev.ack),
            _ => None,
        }
    }
}

pub fn ibc_event_try_from_abci_event(abci_event: &AbciEvent) -> Result<IbcEvent, TypesError> {
    match abci_event.kind.parse() {
        Ok(IbcEventType::CreateClient) => {
            let create_attributes = extract_attributes_from_client_event(abci_event)?;
            let create_client_event = ClientEvents::CreateClient(create_attributes);
            Ok(IbcEvent::CreateClient(create_client_event))
        }
        Ok(IbcEventType::UpdateClient) => Ok(IbcEvent::UpdateClient(
            update_client_try_from_abci_event(abci_event)?,
        )),
        Ok(IbcEventType::UpgradeClient) => Ok(IbcEvent::UpgradeClient(
            upgrade_client_try_from_abci_event(abci_event)?,
        )),
        // Ok(IbcEventType::ClientMisbehaviour) => Ok(IbcEvent::ClientMisbehaviour(
        //     client_misbehaviour_try_from_abci_event(abci_event).map_err(IbcEventError::client)?,
        // )),
        Ok(IbcEventType::OpenInitConnection) => Ok(IbcEvent::OpenInitConnection(
            connection_open_init_try_from_abci_event(abci_event)?,
        )),
        Ok(IbcEventType::OpenTryConnection) => Ok(IbcEvent::OpenTryConnection(
            connection_open_try_try_from_abci_event(abci_event)?,
        )),
        Ok(IbcEventType::OpenAckConnection) => Ok(IbcEvent::OpenAckConnection(
            connection_open_ack_try_from_abci_event(abci_event)?,
        )),
        Ok(IbcEventType::OpenConfirmConnection) => Ok(IbcEvent::OpenConfirmConnection(
            connection_open_confirm_try_from_abci_event(abci_event)?,
        )),
        // Ok(IbcEventType::OpenInitChannel) => Ok(IbcEvent::OpenInitChannel(
        //     channel_open_init_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::OpenTryChannel) => Ok(IbcEvent::OpenTryChannel(
        //     channel_open_try_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::OpenAckChannel) => Ok(IbcEvent::OpenAckChannel(
        //     channel_open_ack_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::OpenConfirmChannel) => Ok(IbcEvent::OpenConfirmChannel(
        //     channel_open_confirm_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::CloseInitChannel) => Ok(IbcEvent::CloseInitChannel(
        //     channel_close_init_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::CloseConfirmChannel) => Ok(IbcEvent::CloseConfirmChannel(
        //     channel_close_confirm_try_from_abci_event(abci_event)
        //         .map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::SendPacket) => Ok(IbcEvent::SendPacket(
        //     send_packet_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::WriteAck) => Ok(IbcEvent::WriteAcknowledgement(
        //     write_acknowledgement_try_from_abci_event(abci_event)
        //         .map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::AckPacket) => Ok(IbcEvent::AcknowledgePacket(
        //     acknowledge_packet_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::Timeout) => Ok(IbcEvent::TimeoutPacket(
        //     timeout_packet_try_from_abci_event(abci_event).map_err(IbcEventError::channel)?,
        // )),
        // Ok(IbcEventType::IncentivizedPacket) => Ok(IbcEvent::IncentivizedPacket(
        //     IncentivizedPacket::try_from(&abci_event.attributes[..]).map_err(IbcEventError::fee)?,
        // )),
        // Ok(IbcEventType::DistributionFee) => Ok(IbcEvent::DistributeFeePacket(
        //     DistributeFeePacket::try_from(&abci_event.attributes[..])
        //         .map_err(IbcEventError::fee)?,
        // )),
        // Ok(IbcEventType::CrossChainQuery) => Ok(IbcEvent::CrossChainQueryPacket(
        //     CrossChainQueryPacket::try_from(&abci_event.attributes[..])
        //         .map_err(IbcEventError::cross_chain_query)?,
        // )),
        _ => Err(TypesError::unsupported_abci_event(abci_event.kind.clone())),
    }
}

pub fn create_client_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ClientEvents::CreateClient, TypesError> {
    extract_attributes_from_client_event(abci_event).map(ClientEvents::CreateClient)
}

pub fn update_client_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ClientEvents::UpdateClient, TypesError> {
    extract_attributes_from_client_event(abci_event).map(|attributes| ClientEvents::UpdateClient {
        common: attributes,
        header: extract_header_from_abci_event(abci_event).ok(),
    })
}

pub fn upgrade_client_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ClientEvents::UpgradeClient, TypesError> {
    extract_attributes_from_client_event(abci_event).map(ClientEvents::UpgradeClient)
}

fn extract_attributes_from_client_event(event: &AbciEvent) -> Result<ClientAttributes, TypesError> {
    let mut attr = ClientAttributes::default();

    let decoded_attributes = decode_attributes(event.attributes.clone())?;
    println!("extract: {:?}", decoded_attributes);

    for tag in decoded_attributes {
        let key = tag.key.as_str();
        let value = tag.value.as_str();
        // println!("key: {}; value: {}", key, value);
        match key {
            ClientEvents::CLIENT_ID_ATTRIBUTE_KEY => attr.client_id = value.parse()?,
            ClientEvents::CLIENT_TYPE_ATTRIBUTE_KEY => attr.client_type = value.parse()?,
            ClientEvents::CONSENSUS_HEIGHT_ATTRIBUTE_KEY => {
                attr.consensus_height = value.parse()?
            }
            _ => {}
        }
    }

    Ok(attr)
}

pub fn extract_header_from_abci_event(event: &AbciEvent) -> Result<AnyHeader, TypesError> {
    for tag in &event.attributes {
        if tag.key == HEADER_ATTRIBUTE_KEY {
            let header_bytes =
                hex::decode(tag.value.to_lowercase()).map_err(TypesError::hex_decode)?;
            return decode_header(&header_bytes);
        }
    }

    Err(TypesError::abci_event_missing_raw_header())
}

pub fn connection_open_init_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ConnectionEvents::OpenInit, TypesError> {
    extract_attributes_from_connection_event(abci_event).map(ConnectionEvents::OpenInit)
}

pub fn connection_open_try_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ConnectionEvents::OpenTry, TypesError> {
    extract_attributes_from_connection_event(abci_event).map(ConnectionEvents::OpenTry)
}

pub fn connection_open_ack_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ConnectionEvents::OpenAck, TypesError> {
    extract_attributes_from_connection_event(abci_event).map(ConnectionEvents::OpenAck)
}

pub fn connection_open_confirm_try_from_abci_event(
    abci_event: &AbciEvent,
) -> Result<ConnectionEvents::OpenConfirm, TypesError> {
    extract_attributes_from_connection_event(abci_event).map(ConnectionEvents::OpenConfirm)
}

fn extract_attributes_from_connection_event(
    event: &AbciEvent,
) -> Result<ConnectionAttributes, TypesError> {
    let mut attr = ConnectionAttributes::default();

    let decoded_attributes = decode_attributes(event.attributes.clone())?;
    println!("extract: {:?}", decoded_attributes);

    for tag in decoded_attributes {
        let key = tag.key.as_str();
        let value = tag.value.as_str();
        match key {
            ConnectionEvents::CONN_ID_ATTRIBUTE_KEY => {
                attr.connection_id = value.parse().ok();
            }
            ConnectionEvents::CLIENT_ID_ATTRIBUTE_KEY => {
                attr.client_id = value.parse()?;
            }
            ConnectionEvents::COUNTERPARTY_CONN_ID_ATTRIBUTE_KEY => {
                attr.counterparty_connection_id = value.parse().ok();
            }
            ConnectionEvents::COUNTERPARTY_CLIENT_ID_ATTRIBUTE_KEY => {
                attr.counterparty_client_id = value.parse()?;
            }
            _ => {}
        }
    }

    Ok(attr)
}

fn channel_extract_attributes_from_tx(event: &AbciEvent) -> Result<ChannelAttributes, TypesError> {
    let mut attr = ChannelAttributes::default();

    for tag in &event.attributes {
        let key = tag.key.as_str();
        let value = tag.value.as_str();
        match key {
            ChannelEvents::PORT_ID_ATTRIBUTE_KEY => {
                attr.port_id = value.parse()?;
            }
            ChannelEvents::CHANNEL_ID_ATTRIBUTE_KEY => {
                attr.channel_id = value.parse().ok();
            }
            ChannelEvents::CONNECTION_ID_ATTRIBUTE_KEY => {
                attr.connection_id = value.parse()?;
            }
            ChannelEvents::COUNTERPARTY_PORT_ID_ATTRIBUTE_KEY => {
                attr.counterparty_port_id = value.parse()?;
            }
            ChannelEvents::COUNTERPARTY_CHANNEL_ID_ATTRIBUTE_KEY => {
                attr.counterparty_channel_id = value.parse().ok();
            }
            _ => {}
        }
    }

    Ok(attr)
}

pub fn decode_attributes(
    attributes: Vec<EventAttribute>,
) -> Result<Vec<EventAttribute>, TypesError> {
    let mut decoded_attributes: Vec<EventAttribute> = vec![];

    for attribute in attributes {
        let decoded_key = base64::decode_to_string(attribute.key)
            .map_err(|e| TypesError::attributes_decode(e))?;
        let decoded_value = base64::decode_to_string(attribute.value)
            .map_err(|e| TypesError::attributes_decode(e))?;

        let decoded_attribute = EventAttribute::from((decoded_key, decoded_value, attribute.index));
        decoded_attributes.push(decoded_attribute)
    }

    Ok(decoded_attributes)
}
