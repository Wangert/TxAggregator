use std::time::Duration;

use ibc_proto::google::protobuf::Any;
use tracing::{debug, error, info, warn};
use types::{
    ibc_core::{
        ics03_connection::{
            connection::{Counterparty, State},
            error::ConnectionError,
            message::MsgConnectionOpenInit,
        },
        ics24_host::identifier::{ClientId, ConnectionId},
    },
    ibc_events::IbcEvent, message::Msg,
};

use crate::{chain::CosmosChain, error::Error};

#[derive(Debug, Clone)]
pub struct ConnectionSide {
    pub chain: CosmosChain,
    pub client_id: ClientId,
    pub connection_id: ConnectionId,
}

impl ConnectionSide {
    pub fn new(chain: CosmosChain, client_id: ClientId) -> Self {
        Self {
            chain,
            client_id,
            connection_id: ConnectionId::default(),
        }
    }

    pub fn chain(&self) -> CosmosChain {
        self.chain.clone()
    }

    pub fn client_id(&self) -> ClientId {
        self.client_id.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub side_a: ConnectionSide,
    pub side_b: ConnectionSide,
    pub delay_period: Duration,
}

impl Connection {
    pub fn new(side_a: ConnectionSide, side_b: ConnectionSide, delay_period: Duration) -> Self {
        Self { side_a, side_b, delay_period }
    }

    pub fn source_chain(&self) -> CosmosChain {
        self.side_a.chain.clone()
    }

    pub fn target_chain(&self) -> CosmosChain {
        self.side_b.chain.clone()
    }

    pub fn source_chain_client_id(&self) -> ClientId {
        self.side_a.client_id()
    }

    pub fn target_chain_client_id(&self) -> ClientId {
        self.side_b.client_id()
    }

    /// Sends a connection open handshake message.
    /// The message sent depends on the chain status of the connection ends.
    // fn do_conn_open_handshake(&mut self) -> Result<(), Error> {
    //     let (a_state, b_state) = self.update_connection_and_query_states()?;
    //     debug!(
    //         "do_conn_open_handshake with connection end states: {}, {}",
    //         a_state, b_state
    //     );

    //     match (a_state, b_state) {
    //         // send the Init message to chain a (source)
    //         (State::Uninitialized, State::Uninitialized) => {
    //             let event = self
    //                 .flipped()
    //                 .build_connection_open_init_and_send()
    //                 .map_err(|e| {
    //                     error!("failed ConnOpenInit {:?}: {}", self.side_a, e);
    //                     e
    //                 })?;
    //             let connection_id = extract_connection_id(&event)?;
    //             self.side_a.connection_id = Some(connection_id.clone());
    //         }

    //         // send the Try message to chain a (source)
    //         (State::Uninitialized, State::Init) | (State::Init, State::Init) => {
    //             let event = self.flipped().build_conn_try_and_send().map_err(|e| {
    //                 error!("failed ConnOpenTry {:?}: {}", self.side_a, e);
    //                 e
    //             })?;

    //             let connection_id = extract_connection_id(&event)?;
    //             self.side_a.connection_id = Some(connection_id.clone());
    //         }

    //         // send the Try message to chain b (destination)
    //         (State::Init, State::Uninitialized) => {
    //             let event = self.build_conn_try_and_send().map_err(|e| {
    //                 error!("failed ConnOpenTry {:?}: {}", self.side_b, e);
    //                 e
    //             })?;

    //             let connection_id = extract_connection_id(&event)?;
    //             self.side_b.connection_id = Some(connection_id.clone());
    //         }

    //         // send the Ack message to chain a (source)
    //         (State::Init, State::TryOpen) | (State::TryOpen, State::TryOpen) => {
    //             self.flipped().build_conn_ack_and_send().map_err(|e| {
    //                 error!("failed ConnOpenAck {:?}: {}", self.side_a, e);
    //                 e
    //             })?;
    //         }

    //         // send the Ack message to chain b (destination)
    //         (State::TryOpen, State::Init) => {
    //             self.build_conn_ack_and_send().map_err(|e| {
    //                 error!("failed ConnOpenAck {:?}: {}", self.side_b, e);
    //                 e
    //             })?;
    //         }

    //         // send the Confirm message to chain b (destination)
    //         (State::Open, State::TryOpen) => {
    //             self.build_conn_confirm_and_send().map_err(|e| {
    //                 error!("failed ConnOpenConfirm {:?}: {}", self.side_a, e);
    //                 e
    //             })?;
    //         }

    //         // send the Confirm message to chain a (source)
    //         (State::TryOpen, State::Open) => {
    //             self.flipped().build_conn_confirm_and_send().map_err(|e| {
    //                 error!("failed ConnOpenConfirm {:?}: {}", self.side_a, e);
    //                 e
    //             })?;
    //         }

    //         (State::Open, State::Open) => {
    //             info!("connection handshake already finished for {:?}", self);
    //             return Ok(());
    //         }

    //         (a_state, b_state) => {
    //             warn!(
    //                 "do_conn_open_handshake does not handle connection end state combination: \
    //                 {}-{}, {}-{}. will retry to account for RPC node data availability issues.",
    //                 self.side_a.chain.id(),
    //                 a_state,
    //                 self.side_b.chain.id(),
    //                 b_state
    //             );
    //         }
    //     }
    //     Err(Error::handshake_finalize())
    // }

    // pub fn build_connection_open_init(&self) -> Result<Vec<Any>, Error> {
    //     // Get signer
    //     let signer = self
    //         .target_chain()
    //         .account()
    //         .get_signer()?;

    //     let prefix = self
    //         .source_chain()
    //         .query_commitment_prefix()
    //         .map_err(|e| Error::chain_query(self.source_chain().id(), e))?;

    //     let counterparty = Counterparty::new(self.source_chain_client_id().clone(), None, prefix);

    //     let version = self
    //         .target_chain()
    //         .query_compatible_versions()[0]
    //         .clone();

    //     // Build the domain type message
    //     let new_msg = MsgConnectionOpenInit {
    //         client_id: self.target_chain_client_id().clone(),
    //         counterparty,
    //         version: Some(version),
    //         delay_period: self.delay_period,
    //         signer,
    //     };

    //     Ok(vec![new_msg.to_any()])
    // }

    // pub fn build_connection_open_init_and_send(&self) -> Result<IbcEvent, Error> {
    //     let msgs = self.build_connection_open_init()?;

    //     // let tm = TrackedMsgs::new_static(dst_msgs, "ConnectionOpenInit");
    //     let events = self
    //         .target_chain()
    //         .send_messages_and_wait_commit(msgs)?;

    //     // Find the relevant event for connection init
    //     let result = events
    //         .into_iter()
    //         .find(|event_with_height| {
    //             matches!(event_with_height.event, IbcEvent::OpenInitConnection(_))
    //                 || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
    //         })
    //         .ok_or_else(Error::missing_connection_init_event)?;

    //     // TODO - make chainError an actual error
    //     match &result.event {
    //         IbcEvent::OpenInitConnection(_) => {
    //             info!("🥂 {} => {}", self.target_chain().id(), result);
    //             Ok(result.event)
    //         }
    //         IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
    //         _ => Err(Error::invalid_event(result.event)),
    //     }
    // }

    pub fn flipped(&self) -> Self {
        Self {
            side_a: self.side_b.clone(),
            side_b: self.side_a.clone(),
            delay_period: self.delay_period.clone(),
        }
    }
}
