use std::{thread, time::Duration};

use ibc_proto::google::protobuf::Any;
use log::trace;
use tracing::{debug, error, info, warn};
use types::{
    ibc_core::{
        ics02_client::{header::AnyHeader, height::Height, update_client::MsgUpdateClient},
        ics03_connection::{
            connection::{Counterparty, State},
            error::ConnectionError,
            events::extract_connection_id,
            message::{MsgConnectionOpenInit, MsgConnectionOpenTry},
        },
        ics24_host::identifier::{ClientId, ConnectionId},
    },
    ibc_events::IbcEvent,
    message::Msg,
};

use crate::{chain::CosmosChain, common::QueryHeight, error::Error, light_client::Verified};

/// Enumeration of proof carrying ICS3 message, helper for relayer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionMsgType {
    OpenTry,
    OpenAck,
    OpenConfirm,
}

#[derive(Debug, Clone)]
pub struct ConnectionSide {
    pub chain: CosmosChain,
    pub client_id: ClientId,
    pub connection_id: Option<ConnectionId>,
}

impl ConnectionSide {
    pub fn new(chain: CosmosChain, client_id: ClientId) -> Self {
        Self {
            chain,
            client_id,
            connection_id: None,
        }
    }

    pub fn chain(&self) -> CosmosChain {
        self.chain.clone()
    }

    pub fn client_id(&self) -> ClientId {
        self.client_id.clone()
    }

    pub fn connection_id(&self) -> Option<ConnectionId> {
        self.connection_id.clone()
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
        Self {
            side_a,
            side_b,
            delay_period,
        }
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

    pub async fn build_update_client_on_source_chain(
        &self,
        target_height: Height,
    ) -> Result<Vec<Any>, Error> {
        trace!("build_update_client_on_source_chain");

        let client_id = self.source_chain_client_id();
        // query consensus state on source chain
        let client_consensus_state_on_source = self.source_chain().query_client_consensus_state(
            &client_id,
            target_height,
            QueryHeight::Latest,
            false,
        );

        if let Ok(_) = client_consensus_state_on_source {
            debug!("consensus state already exists at height {target_height}, skipping update");
            return Ok(vec![]);
        }

        let target_chain_latest_height = || self.target_chain().query_latest_height();

        while target_chain_latest_height()? < target_height {
            thread::sleep(Duration::from_millis(100));
        }

        // validate client state
        let (client_state, _) =
            self.source_chain()
                .query_client_state(&client_id, QueryHeight::Latest, true)?;
        let client_state_validate = self
            .source_chain()
            .validate_client_state(&client_id, &client_state);

        if let Some(e) = client_state_validate {
            return Err(e);
        }

        // Obtain the required block based on the target block height and client_state
        let verified_blocks = self
            .target_chain()
            .query_light_blocks(&client_state, target_height)?;

        let trusted_height =
            self.source_chain()
                .query_trusted_height(target_height, &client_id, &client_state)?;

        let (target_header, support_headers) = self
            .target_chain()
            .adjust_headers(
                trusted_height,
                verified_blocks.target,
                verified_blocks.supporting,
            )
            .map(|(target_header, support_headers)| {
                let header = AnyHeader::from(target_header);
                let support: Vec<AnyHeader> = support_headers
                    .into_iter()
                    .map(|h| AnyHeader::from(h))
                    .collect();
                (header, support)
            })?;

        let signer = self.source_chain().account().get_signer()?;

        let mut msgs = vec![];
        for header in support_headers {
            msgs.push(MsgUpdateClient {
                header: header.into(),
                client_id: client_id.clone(),
                signer: signer.clone(),
            });
        }

        msgs.push(MsgUpdateClient {
            header: target_header.into(),
            signer,
            client_id: client_id.clone(),
        });

        let encoded_messages = msgs.into_iter().map(Msg::to_any).collect::<Vec<Any>>();

        return Ok(encoded_messages);
    }

    pub async fn build_update_client_on_target_chain(
        &self,
        target_height: Height,
    ) -> Result<Vec<Any>, Error> {
        trace!("build_update_client_on_target_chain");

        let client_id = self.target_chain_client_id();
        // query consensus state on source chain
        let client_consensus_state_on_target = self.target_chain().query_client_consensus_state(
            &client_id,
            target_height,
            QueryHeight::Latest,
            false,
        );

        if let Ok(_) = client_consensus_state_on_target {
            debug!("consensus state already exists at height {target_height}, skipping update");
            return Ok(vec![]);
        }

        let source_chain_latest_height = || self.source_chain().query_latest_height();

        while source_chain_latest_height()? < target_height {
            thread::sleep(Duration::from_millis(100));
        }

        // validate client state
        let (client_state, _) =
            self.target_chain()
                .query_client_state(&client_id, QueryHeight::Latest, true)?;
        let client_state_validate = self
            .target_chain()
            .validate_client_state(&client_id, &client_state);

        if let Some(e) = client_state_validate {
            return Err(e);
        }

        // Obtain the required block based on the target block height and client_state
        let verified_blocks = self
            .source_chain()
            .query_light_blocks(&client_state, target_height)?;

        let trusted_height =
            self.source_chain()
                .query_trusted_height(target_height, &client_id, &client_state)?;

        let (target_header, support_headers) = self
            .source_chain()
            .adjust_headers(
                trusted_height,
                verified_blocks.target,
                verified_blocks.supporting,
            )
            .map(|(target_header, support_headers)| {
                let header = AnyHeader::from(target_header);
                let support: Vec<AnyHeader> = support_headers
                    .into_iter()
                    .map(|h| AnyHeader::from(h))
                    .collect();
                (header, support)
            })?;

        let signer = self.target_chain().account().get_signer()?;

        let mut msgs = vec![];
        for header in support_headers {
            msgs.push(MsgUpdateClient {
                header: header.into(),
                client_id: client_id.clone(),
                signer: signer.clone(),
            });
        }

        msgs.push(MsgUpdateClient {
            header: target_header.into(),
            signer,
            client_id: client_id.clone(),
        });

        let encoded_messages = msgs.into_iter().map(Msg::to_any).collect::<Vec<Any>>();

        return Ok(encoded_messages);
    }

    // Sends a connection open handshake message.
    // The message sent depends on the chain status of the connection ends.
    async fn do_conn_open_handshake(&mut self) -> Result<(), Error> {
        let (a_state, b_state) = self.update_connection_state()?;
        debug!(
            "do_conn_open_handshake with connection end states: {}, {}",
            a_state, b_state
        );

        match (a_state, b_state) {
            // send the Init message to chain a (source)
            (State::Uninitialized, State::Uninitialized) => {
                let event = self
                    .flipped()
                    .build_connection_open_init_and_send()
                    .map_err(|e| {
                        error!("failed ConnOpenInit {:?}: {}", self.side_a, e);
                        e
                    })?;
                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_a.connection_id = Some(connection_id.clone());
            }

            // send the Try message to chain a (source)
            (State::Uninitialized, State::Init) | (State::Init, State::Init) => {
                let event = self.flipped().build_connection_open_try_and_send().await?;

                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_a.connection_id = Some(connection_id.clone());
            }

            // send the Try message to chain b (destination)
            (State::Init, State::Uninitialized) => {
                let event = self.build_connection_open_try_and_send().await?;

                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_b.connection_id = Some(connection_id.clone());
            }

            // // send the Ack message to chain a (source)
            // (State::Init, State::TryOpen) | (State::TryOpen, State::TryOpen) => {
            //     self.flipped().build_conn_ack_and_send().map_err(|e| {
            //         error!("failed ConnOpenAck {:?}: {}", self.side_a, e);
            //         e
            //     })?;
            // }

            // // send the Ack message to chain b (destination)
            // (State::TryOpen, State::Init) => {
            //     self.build_conn_ack_and_send().map_err(|e| {
            //         error!("failed ConnOpenAck {:?}: {}", self.side_b, e);
            //         e
            //     })?;
            // }

            // // send the Confirm message to chain b (destination)
            // (State::Open, State::TryOpen) => {
            //     self.build_conn_confirm_and_send().map_err(|e| {
            //         error!("failed ConnOpenConfirm {:?}: {}", self.side_a, e);
            //         e
            //     })?;
            // }

            // // send the Confirm message to chain a (source)
            // (State::TryOpen, State::Open) => {
            //     self.flipped().build_conn_confirm_and_send().map_err(|e| {
            //         error!("failed ConnOpenConfirm {:?}: {}", self.side_a, e);
            //         e
            //     })?;
            // }
            (State::Open, State::Open) => {
                info!("connection handshake already finished for {:?}", self);
                return Ok(());
            }

            (a_state, b_state) => {
                warn!(
                    "do_conn_open_handshake does not handle connection end state combination: \
                    {}-{}, {}-{}. will retry to account for RPC node data availability issues.",
                    self.side_a.chain.id(),
                    a_state,
                    self.side_b.chain.id(),
                    b_state
                );
            }
        }
        Err(Error::handshake_continue())
    }

    pub fn update_connection_state(&mut self) -> Result<(State, State), Error> {
        let old_con_a_id = self.side_a.connection_id();
        let old_con_b_id = self.side_b.connection_id();

        let (a_connection, _) = self.source_chain().query_connection(
            old_con_a_id.as_ref().ok_or_else(Error::empty_connection_id)?,
            QueryHeight::Latest,
            false,
        )?;
        let a_counterparty_id = a_connection.counterparty().connection_id();

        if a_counterparty_id.is_some() && a_counterparty_id != old_con_b_id.as_ref() {
            // warn!(
            //     "updating the expected {} of side_b({}) since it is different than the \
            //     counterparty of {}: {}, on {}. This is typically caused by crossing handshake \
            //     messages in the presence of multiple relayers.",
            //     PrettyOption(&relayer_b_id),
            //     self.b_chain().id(),
            //     PrettyOption(&relayer_a_id),
            //     PrettyOption(&a_counterparty_id),
            //     self.a_chain().id(),
            // );
            self.side_b.connection_id = a_counterparty_id.cloned();
        }

        let updated_con_b_id = self.side_b.connection_id();
        let (b_connection, _) = self.target_chain().query_connection(
            old_con_b_id.as_ref().ok_or_else(Error::empty_connection_id)?,
            crate::common::QueryHeight::Latest,
            false,
        )?;
        let b_counterparty_id = b_connection.counterparty().connection_id();

        if b_counterparty_id.is_some() && b_counterparty_id != old_con_a_id.as_ref() {
            if updated_con_b_id == old_con_b_id {
                self.side_a.connection_id = b_counterparty_id.cloned();
            } else {
                panic!(
                    "mismatched connection ids in connection ends: {} - {:?} and {} - {:?}",
                    self.source_chain().id(),
                    a_connection,
                    self.target_chain().id(),
                    b_connection,
                );
            }
        }
        Ok((*a_connection.state(), *b_connection.state()))
    }

    pub fn build_connection_open_init(&self) -> Result<Vec<Any>, Error> {
        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        let prefix = self.source_chain().query_commitment_prefix()?;

        let counterparty = Counterparty::new(self.source_chain_client_id().clone(), None, prefix);

        let version = self.target_chain().query_compatible_versions()[0].clone();

        // Build the domain type message
        let new_msg = MsgConnectionOpenInit {
            client_id: self.target_chain_client_id().clone(),
            counterparty,
            version: Some(version),
            delay_period: self.delay_period,
            signer,
        };

        Ok(vec![new_msg.to_any()])
    }

    pub fn build_connection_open_init_and_send(&self) -> Result<IbcEvent, Error> {
        let msgs = self.build_connection_open_init()?;

        // let tm = TrackedMsgs::new_static(dst_msgs, "ConnectionOpenInit");
        let events = self.target_chain().send_messages_and_wait_commit(msgs)?;

        // Find the relevant event for connection init
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenInitConnection(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(Error::missing_connection_init_event)?;

        // TODO - make chainError an actual error
        match &result.event {
            IbcEvent::OpenInitConnection(_) => {
                info!("🥂 {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    /// Attempts to build a MsgConnOpenTry.
    ///
    /// Return the messages and the app height the destination chain must reach
    /// before we send the messages.
    pub async fn build_connection_open_try(&self) -> Result<(Vec<Any>, Height), Error> {
        let src_connection_id = self
            .side_a
            .connection_id()
            .ok_or_else(Error::empty_connection_id)?;

        let (src_connection, _) = self.source_chain().query_connection(
            &src_connection_id,
            QueryHeight::Latest,
            false,
        )?;

        // Cross-check the delay_period
        let delay = if src_connection.delay_period() != self.delay_period {
            warn!("`delay_period` for ConnectionEnd @{} is {}s; delay period on local Connection object is set to {}s",
                self.source_chain().id(), src_connection.delay_period().as_secs_f64(), self.delay_period.as_secs_f64());

            warn!(
                "Overriding delay period for local connection object to {}s",
                src_connection.delay_period().as_secs_f64()
            );

            src_connection.delay_period()
        } else {
            self.delay_period
        };

        // Build add send the message(s) for updating client on source
        let src_client_target_height = self.target_chain().query_latest_height()?;
        let update_client_msgs = self
            .build_update_client_on_source_chain(src_client_target_height)
            .await?;

        // let tm =
        //     TrackedMsgs::new_static(client_msgs, "update client on source for ConnectionOpenTry");
        self.source_chain()
            .send_messages_and_wait_commit(update_client_msgs)?;

        let query_height = self.source_chain().query_latest_height()?;
        let (client_state, proofs) = self
            .source_chain()
            .build_connection_proofs_and_client_state(
                ConnectionMsgType::OpenTry,
                &src_connection_id,
                &self.side_a.client_id(),
                query_height,
            )?;

        // Build message(s) for updating client on destination
        let mut msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        let counterparty_versions = if src_connection.versions().is_empty() {
            self.source_chain().query_compatible_versions()
        } else {
            src_connection.versions().to_vec()
        };

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        let prefix = self.source_chain().query_commitment_prefix()?;

        let counterparty = Counterparty::new(
            self.side_a.client_id().clone(),
            self.side_a.connection_id(),
            prefix,
        );

        let previous_connection_id = if src_connection.counterparty().connection_id.is_none() {
            self.side_b.connection_id()
        } else {
            src_connection.counterparty().connection_id.clone()
        };

        let new_msg = MsgConnectionOpenTry {
            client_id: self.side_b.client_id(),
            client_state: client_state.map(Into::into),
            previous_connection_id,
            counterparty,
            counterparty_versions,
            proofs,
            delay_period: delay,
            signer,
        };

        msgs.push(new_msg.to_any());

        Ok((msgs, src_client_target_height))
    }

    pub async fn build_connection_open_try_and_send(&self) -> Result<IbcEvent, Error> {
        let (con_open_try_msgs, src_client_target_height) =
            self.build_connection_open_try().await?;

        // Wait for the height of the application on the target chain to be higher than
        // the height of the consensus state included in the proofs.
        self.wait_for_target_chain_height_higher_than_consensus_height(src_client_target_height)?;

        // let tm = TrackedMsgs::new_static(dst_msgs, "ConnectionOpenTry");

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(con_open_try_msgs)?;

        // Find the relevant event for connection try transaction
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenTryConnection(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| {
                Error::connection_error(ConnectionError::missing_connection_try_event())
            })?;

        match &result.event {
            IbcEvent::OpenTryConnection(_) => {
                info!("🥂 {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    pub fn flipped(&self) -> Self {
        Self {
            side_a: self.side_b.clone(),
            side_b: self.side_a.clone(),
            delay_period: self.delay_period.clone(),
        }
    }

    /// Wait for the application on target chain to advance beyond `consensus_height`.
    fn wait_for_target_chain_height_higher_than_consensus_height(
        &self,
        consensus_height: Height,
    ) -> Result<(), Error> {
        let target_chain_latest_height = || {
            self.target_chain()
                .query_latest_height()
        };

        while consensus_height >= target_chain_latest_height()? {
            warn!(
                "client consensus proof height too high, \
                 waiting for destination chain to advance beyond {}",
                consensus_height
            );

            thread::sleep(Duration::from_millis(500));
        }

        Ok(())
    }
}
