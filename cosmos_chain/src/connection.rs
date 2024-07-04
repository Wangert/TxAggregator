use std::{fmt::Display, thread, time::Duration};

use ibc_proto::google::protobuf::Any;
use log::trace;
use tracing::{debug, error, info, warn};
use types::{
    error::TypesError,
    ibc_core::{
        ics02_client::{header::AnyHeader, height::Height, update_client::MsgUpdateClient},
        ics03_connection::{
            connection::{ConnectionEnd, Counterparty, State},
            error::ConnectionError,
            events::extract_connection_id,
            message::{
                MsgConnectionOpenAck, MsgConnectionOpenConfirm, MsgConnectionOpenInit,
                MsgConnectionOpenTry,
            },
        },
        ics24_host::identifier::{
            ClientId, ConnectionId, AGGRELITE_CLIENT_PREFIX, TENDERMINT_CLIENT_PREFIX,
        },
    },
    ibc_events::IbcEvent,
    light_clients::{client_type::ClientStateType, header_type::AdjustHeadersType},
    message::Msg,
    timestamp::ZERO_DURATION,
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

    pub fn source_chain(&self) -> &CosmosChain {
        &self.side_a.chain
    }

    pub fn target_chain(&self) -> &CosmosChain {
        &self.side_b.chain
    }

    pub fn source_chain_client_id(&self) -> ClientId {
        self.side_a.client_id()
    }

    pub fn target_chain_client_id(&self) -> ClientId {
        self.side_b.client_id()
    }

    pub fn source_chain_connection_id(&self) -> Option<ConnectionId> {
        self.side_a.connection_id()
    }

    pub fn target_chain_connection_id(&self) -> Option<ConnectionId> {
        self.side_b.connection_id()
    }

    pub async fn build_update_client_on_source_chain(
        &self,
        target_height: Height,
    ) -> Result<Vec<Any>, Error> {
        trace!("build_update_client_on_source_chain");

        let client_id = self.source_chain_client_id();
        // query consensus state on source chain
        let client_consensus_state_on_source = self
            .source_chain()
            .query_client_consensus_state(&client_id, target_height, QueryHeight::Latest, false)
            .await;

        if let Ok(_) = client_consensus_state_on_source {
            debug!("consensus state already exists at height {target_height}, skipping update");
            return Ok(vec![]);
        }

        let target_chain = self.target_chain().clone();
        let target_chain_latest_height = || target_chain.query_latest_height();

        while target_chain_latest_height().await? < target_height {
            thread::sleep(Duration::from_millis(100));
        }

        // validate client state
        let (client_state, _) = self
            .source_chain()
            .query_client_state(&client_id, QueryHeight::Latest, true)
            .await?;
        let client_state_validate = self
            .source_chain()
            .validate_client_state(&client_id, client_state.clone())
            .await;

        if let Some(e) = client_state_validate {
            return Err(e);
        }

        // Obtain the required block based on the target block height and client_state
        let verified_blocks = self
            .target_chain()
            .query_light_blocks(client_state.clone(), target_height)
            .await?;

        let trusted_height = self
            .source_chain()
            .query_trusted_height(target_height, &client_id, client_state)
            .await?;

        let (target_header, support_headers) = if client_id.check_type(TENDERMINT_CLIENT_PREFIX) {
            self.target_chain().adjust_headers(
                trusted_height,
                verified_blocks.target,
                verified_blocks.supporting,
                TENDERMINT_CLIENT_PREFIX,
            )
            .await
            .map(|adjust_headers| match adjust_headers {
                AdjustHeadersType::Tendermint(headers) => {
                    let header = AnyHeader::from(headers.target_header);
                    let support: Vec<AnyHeader> = headers
                        .supporting_headers
                        .into_iter()
                        .map(|h| AnyHeader::from(h))
                        .collect();
                    (header, support)
                }
                AdjustHeadersType::Aggrelite(headers) => {
                    let header = AnyHeader::from(headers.target_header);
                    let support: Vec<AnyHeader> = headers
                        .supporting_headers
                        .into_iter()
                        .map(|h| AnyHeader::from(h))
                        .collect();
                    (header, support)
                }
            })?
        } else {
            self.target_chain().adjust_headers(
                trusted_height,
                verified_blocks.target,
                verified_blocks.supporting,
                AGGRELITE_CLIENT_PREFIX,
            )
            .await
            .map(|adjust_headers| match adjust_headers {
                AdjustHeadersType::Tendermint(headers) => {
                    let header = AnyHeader::from(headers.target_header);
                    let support: Vec<AnyHeader> = headers
                        .supporting_headers
                        .into_iter()
                        .map(|h| AnyHeader::from(h))
                        .collect();
                    (header, support)
                }
                AdjustHeadersType::Aggrelite(headers) => {
                    let header = AnyHeader::from(headers.target_header);
                    let support: Vec<AnyHeader> = headers
                        .supporting_headers
                        .into_iter()
                        .map(|h| AnyHeader::from(h))
                        .collect();
                    (header, support)
                }
            })?
        };
        // let (target_header, support_headers) = self
        //     .target_chain()
        //     .adjust_headers(
        //         trusted_height,
        //         verified_blocks.target,
        //         verified_blocks.supporting,
        //     )
        //     .await
        //     .map(|(target_header, support_headers)| {
        //         let header = AnyHeader::from(target_header);
        //         let support: Vec<AnyHeader> = support_headers
        //             .into_iter()
        //             .map(|h| AnyHeader::from(h))
        //             .collect();
        //         (header, support)
        //     })?;

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
        let client_consensus_state_on_target = self
            .target_chain()
            .query_client_consensus_state(&client_id, target_height, QueryHeight::Latest, false)
            .await;

        if let Ok(_) = client_consensus_state_on_target {
            debug!("consensus state already exists at height {target_height}, skipping update");
            return Ok(vec![]);
        }

        let source_chain = self.source_chain().clone();
        let source_chain_latest_height = || source_chain.query_latest_height();

        while source_chain_latest_height().await? < target_height {
            thread::sleep(Duration::from_millis(100));
        }

        // validate client state
        let (client_state, _) = self
            .target_chain()
            .query_client_state(&client_id, QueryHeight::Latest, true)
            .await?;
        let client_state_validate = self
            .target_chain()
            .validate_client_state(&client_id, client_state.clone())
            .await;

        if let Some(e) = client_state_validate {
            return Err(e);
        }

        // Obtain the required block based on the target block height and client_state
        let verified_blocks = self
            .source_chain()
            .query_light_blocks(client_state.clone(), target_height)
            .await?;

        let trusted_height = self
            .source_chain()
            .query_trusted_height(target_height, &client_id, client_state)
            .await?;

        let (target_header, support_headers) = if client_id.check_type(TENDERMINT_CLIENT_PREFIX) {
            self.source_chain()
                .adjust_headers(
                    trusted_height,
                    verified_blocks.target,
                    verified_blocks.supporting,
                    TENDERMINT_CLIENT_PREFIX,
                )
                .await
                .map(|adjust_headers| match adjust_headers {
                    AdjustHeadersType::Tendermint(headers) => {
                        let header = AnyHeader::from(headers.target_header);
                        let support: Vec<AnyHeader> = headers
                            .supporting_headers
                            .into_iter()
                            .map(|h| AnyHeader::from(h))
                            .collect();
                        (header, support)
                    }
                    AdjustHeadersType::Aggrelite(headers) => {
                        let header = AnyHeader::from(headers.target_header);
                        let support: Vec<AnyHeader> = headers
                            .supporting_headers
                            .into_iter()
                            .map(|h| AnyHeader::from(h))
                            .collect();
                        (header, support)
                    }
                })?
        } else {
            self.source_chain()
                .adjust_headers(
                    trusted_height,
                    verified_blocks.target,
                    verified_blocks.supporting,
                    AGGRELITE_CLIENT_PREFIX,
                )
                .await
                .map(|adjust_headers| match adjust_headers {
                    AdjustHeadersType::Tendermint(headers) => {
                        let header = AnyHeader::from(headers.target_header);
                        let support: Vec<AnyHeader> = headers
                            .supporting_headers
                            .into_iter()
                            .map(|h| AnyHeader::from(h))
                            .collect();
                        (header, support)
                    }
                    AdjustHeadersType::Aggrelite(headers) => {
                        let header = AnyHeader::from(headers.target_header);
                        let support: Vec<AnyHeader> = headers
                            .supporting_headers
                            .into_iter()
                            .map(|h| AnyHeader::from(h))
                            .collect();
                        (header, support)
                    }
                })?
        };
        // let (target_header, support_headers) = self
        //     .source_chain()
        //     .adjust_headers(
        //         trusted_height,
        //         verified_blocks.target,
        //         verified_blocks.supporting,
        //     )
        //     .await
        //     .map(|(target_header, support_headers)| {
        //         let header = AnyHeader::from(target_header);
        //         let support: Vec<AnyHeader> = support_headers
        //             .into_iter()
        //             .map(|h| AnyHeader::from(h))
        //             .collect();
        //         (header, support)
        //     })?;

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

    fn update_connection_id(
        &mut self,
        client_id: ClientId,
        connection_id: Option<ConnectionId>,
    ) -> Result<(), Error> {
        if self.source_chain_client_id() == client_id {
            self.side_a.connection_id = connection_id;
        } else if self.target_chain_client_id() == client_id {
            self.side_b.connection_id = connection_id;
        } else {
            return Err(Error::connection_handshke_abnormal());
        }

        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), Error> {
        loop {
            let event_result = self.connection_handshake().await;
            match event_result {
                Ok(Some(IbcEvent::OpenInitConnection(e))) => {
                    let init_chain_client_id = e.attributes().client_id.clone();
                    let update_connection_id = e.attributes().connection_id.clone();
                    self.update_connection_id(init_chain_client_id, update_connection_id)?;
                }
                Ok(Some(IbcEvent::OpenTryConnection(e))) => {
                    let try_chain_client_id = e.attributes().client_id.clone();
                    let update_connection_id = e.attributes().connection_id.clone();
                    self.update_connection_id(try_chain_client_id, update_connection_id)?;
                }
                Err(e) if e.to_string() == Error::connection_completed().to_string() => {
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
                _ => {
                    continue;
                }
            }
        }

        Ok(())
    }

    // Sends a connection open handshake message.
    // The message sent depends on the chain status of the connection.
    async fn connection_handshake(&mut self) -> Result<Option<IbcEvent>, Error> {
        info!("【connection handshake】");
        println!("【connection handshake】");
        let (a_state, b_state) = self.update_connection_state().await?;
        debug!(
            "connection_handshake with connection end states: {}, {}",
            a_state, b_state
        );

        let mut ibc_event: Option<IbcEvent> = None;

        println!(
            "【chain_id({}):state({}) - chain_id({}):state({})",
            self.source_chain().id(),
            a_state,
            self.target_chain().id(),
            b_state
        );
        info!(
            "【chain_id({}):state({}) - chain_id({}):state({})",
            self.source_chain().id(),
            a_state,
            self.target_chain().id(),
            b_state
        );

        match (a_state, b_state) {
            // send the OpenInit message to chain a (source)
            (State::Uninitialized, State::Uninitialized) => {
                info!("send a OpenInit message");
                println!("send a OpenInit message");
                let event = self.flipped().build_connection_open_init_and_send().await?;
                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_a.connection_id = Some(connection_id.clone());

                ibc_event = Some(event)
            }

            // send the OpenTry message to chain a (source)
            (State::Uninitialized, State::Init) | (State::Init, State::Init) => {
                info!("send a OpenTry message");
                println!("send a OpenTry message");
                let event = self.flipped().build_connection_open_try_and_send().await?;

                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_a.connection_id = Some(connection_id.clone());

                ibc_event = Some(event)
            }

            // send the OpenTry message to chain b (target)
            (State::Init, State::Uninitialized) => {
                info!("send a OpenInit message");
                println!("send a OpenInit message");
                let event = self.build_connection_open_try_and_send().await?;

                let connection_id =
                    extract_connection_id(&event).map_err(Error::connection_error)?;
                self.side_b.connection_id = Some(connection_id.clone());

                ibc_event = Some(event)
            }

            // send the Ack message to chain a (source)
            (State::Init, State::TryOpen) | (State::TryOpen, State::TryOpen) => {
                info!("send a OpenAck message");
                println!("send a OpenAck message");
                let event = self.flipped().build_connection_open_ack_and_send().await?;

                ibc_event = Some(event)
            }

            // send the Ack message to chain b (target)
            (State::TryOpen, State::Init) => {
                info!("send a OpenAck message");
                println!("send a OpenAck message");
                let event = self.build_connection_open_ack_and_send().await?;

                ibc_event = Some(event)
            }

            // send the Confirm message to chain b (target)
            (State::Open, State::TryOpen) => {
                info!("send a OpenConfirm message");
                println!("send a OpenConfirm message");
                let event = self.build_connection_open_confirm_and_send().await?;

                ibc_event = Some(event)
            }

            // send the Confirm message to chain a (source)
            (State::TryOpen, State::Open) => {
                info!("send a OpenConfirm message");
                println!("send a OpenConfirm message");
                let event = self
                    .flipped()
                    .build_connection_open_confirm_and_send()
                    .await?;

                ibc_event = Some(event)
            }
            (State::Open, State::Open) => {
                info!("connection handshake already finished for {:?}", self);
                println!("connection handshake already finished for {:?}", self);
                return Err(Error::connection_completed());
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
                return Err(Error::connection_state_error());
            }
        }

        println!("@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@");
        println!("Connection Handshake Event: {:#?}", ibc_event);
        // Err(Error::handshake_continue())
        Ok(ibc_event)
    }

    pub async fn update_connection_state(&mut self) -> Result<(State, State), Error> {
        let old_con_a_id = self.source_chain_connection_id();
        let old_con_b_id = self.target_chain_connection_id();

        let a_connection = if let Some(conn_id) = old_con_a_id.as_ref() {
            let (a_connection, _) = self
                .source_chain()
                .query_connection(conn_id, QueryHeight::Latest, true)
                .await?;
            a_connection
        } else {
            ConnectionEnd::default()
        };

        let a_counterparty_id = a_connection.counterparty().connection_id();

        if a_counterparty_id.is_some() && a_counterparty_id != old_con_b_id.as_ref() {
            self.side_b.connection_id = a_counterparty_id.cloned();
        }

        let updated_con_b_id = self.target_chain_connection_id();

        let b_connection = if let Some(conn_id) = updated_con_b_id.as_ref() {
            let (b_connection, _) = self
                .target_chain()
                .query_connection(conn_id, QueryHeight::Latest, true)
                .await?;
            b_connection
        } else {
            ConnectionEnd::default()
        };

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

        println!("open init: {:?}", new_msg);

        Ok(vec![new_msg.to_any()])
    }

    pub async fn build_connection_open_init_and_send(&self) -> Result<IbcEvent, Error> {
        info!("build_connection_open_init_and_send");
        let msgs = self.build_connection_open_init()?;

        // println!("msgs: {:?}", msgs);
        // let tm = TrackedMsgs::new_static(dst_msgs, "ConnectionOpenInit");
        let events = self
            .target_chain()
            .send_messages_and_wait_commit(msgs)
            .await?;

        // println!("ibc events: {:?}", events);
        // Find the relevant event for connection init
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenInitConnection(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(Error::missing_connection_init_event)?;

        match &result.event {
            IbcEvent::OpenInitConnection(_) => {
                info!("🥂 {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }

        // Err(Error::empty_connection_id())
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

        let (src_connection, _) = self
            .source_chain()
            .query_connection(&src_connection_id, QueryHeight::Latest, false)
            .await?;

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
        let src_client_target_height = self.target_chain().query_latest_height().await?;
        let update_client_msgs = self
            .build_update_client_on_source_chain(src_client_target_height)
            .await?;

        // let tm =
        //     TrackedMsgs::new_static(client_msgs, "update client on source for ConnectionOpenTry");
        self.source_chain()
            .send_messages_and_wait_commit(update_client_msgs)
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;
        let (client_state, proofs) = self
            .source_chain()
            .build_connection_proofs_and_client_state(
                ConnectionMsgType::OpenTry,
                &src_connection_id,
                &self.side_a.client_id(),
                query_height,
            )
            .await?;

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
            self.source_chain_connection_id(),
            prefix,
        );

        let previous_connection_id = if src_connection.counterparty().connection_id.is_none() {
            self.target_chain_connection_id()
        } else {
            src_connection.counterparty().connection_id.clone()
        };

        let client_state_any: Option<Any> = match client_state {
            Some(ClientStateType::Tendermint(cs)) => Some(cs.into()),
            Some(ClientStateType::Aggrelite(cs)) => Some(cs.into()),
            _ => None,
        };
        let new_msg = MsgConnectionOpenTry {
            client_id: self.side_b.client_id(),
            client_state: client_state_any,
            previous_connection_id: None,
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
        self.wait_for_target_chain_height_higher_than_consensus_height(src_client_target_height)
            .await?;

        // let tm = TrackedMsgs::new_static(dst_msgs, "ConnectionOpenTry");

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(con_open_try_msgs)
            .await?;

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

    // Attempts to build a MsgConnOpenAck.
    pub async fn build_connection_open_ack(&self) -> Result<(Vec<Any>, Height), Error> {
        let src_connection_id = self
            .side_a
            .connection_id()
            .ok_or_else(Error::empty_connection_id)?;
        let dst_connection_id = self
            .side_b
            .connection_id()
            .ok_or_else(Error::empty_connection_id)?;

        let _expected_dst_connection = self
            .validated_expected_connection(ConnectionMsgType::OpenAck)
            .await?;

        let (src_connection, _) = self
            .source_chain()
            .query_connection(&src_connection_id.clone(), QueryHeight::Latest, false)
            .await?;

        // Update the client of the target chain on the source chain
        let src_client_target_height = self.target_chain().query_latest_height().await?;

        let update_client_msgs_on_source = self
            .build_update_client_on_source_chain(src_client_target_height)
            .await?;

        self.source_chain()
            .send_messages_and_wait_commit(update_client_msgs_on_source)
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;

        let (client_state, proofs) = self
            .source_chain()
            .build_connection_proofs_and_client_state(
                ConnectionMsgType::OpenAck,
                &src_connection_id,
                &self.source_chain_client_id(),
                query_height,
            )
            .await?;

        // Build message(s) for updating client on destination
        let mut msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        let client_state_any: Option<Any> = match client_state {
            Some(ClientStateType::Tendermint(cs)) => Some(cs.into()),
            Some(ClientStateType::Aggrelite(cs)) => Some(cs.into()),
            _ => None,
        };

        let new_msg = MsgConnectionOpenAck {
            connection_id: dst_connection_id.clone(),
            counterparty_connection_id: src_connection_id.clone(),
            client_state: client_state_any,
            proofs,
            version: src_connection.versions()[0].clone(),
            signer,
        };

        msgs.push(new_msg.to_any());

        Ok((msgs, src_client_target_height))
    }

    pub async fn build_connection_open_ack_and_send(&self) -> Result<IbcEvent, Error> {
        let (conn_open_ack_msgs, src_client_target_height) =
            self.build_connection_open_ack().await?;

        // Wait for the height of the target chain to be higher than
        // the height of the consensus state included in the proofs.
        self.wait_for_target_chain_height_higher_than_consensus_height(src_client_target_height)
            .await?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(conn_open_ack_msgs)
            .await?;

        // Find the relevant event for connection ack
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenAckConnection(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| {
                Error::connection_error(ConnectionError::missing_connection_ack_event())
            })?;

        match &result.event {
            IbcEvent::OpenAckConnection(_) => {
                info!("🥂 {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    /// Attempts to build a MsgConnOpenConfirm.
    pub async fn build_connection_open_confirm(&self) -> Result<Vec<Any>, Error> {
        let src_connection_id = self
            .side_a
            .connection_id()
            .ok_or_else(Error::empty_connection_id)?;
        let dst_connection_id = self
            .side_b
            .connection_id()
            .ok_or_else(Error::empty_connection_id)?;

        let _expected_dst_connection = self
            .validated_expected_connection(ConnectionMsgType::OpenConfirm)
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;

        let (_src_connection, _) = self
            .source_chain()
            .query_connection(
                &src_connection_id,
                QueryHeight::Specific(query_height),
                false,
            )
            .await?;

        let (_, proofs) = self
            .source_chain()
            .build_connection_proofs_and_client_state(
                ConnectionMsgType::OpenConfirm,
                &src_connection_id,
                &self.source_chain_client_id(),
                query_height,
            )
            .await?;

        // Build message(s) for updating client on target chain
        let mut msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        let new_msg = MsgConnectionOpenConfirm {
            connection_id: dst_connection_id.clone(),
            proofs,
            signer,
        };

        msgs.push(new_msg.to_any());
        Ok(msgs)
    }

    pub async fn build_connection_open_confirm_and_send(&self) -> Result<IbcEvent, Error> {
        let conn_open_confirm_msgs = self.build_connection_open_confirm().await?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(conn_open_confirm_msgs)
            .await?;

        // Find the relevant event for connection confirm
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenConfirmConnection(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| {
                Error::connection_error(ConnectionError::missing_connection_confirm_event())
            })?;

        match &result.event {
            IbcEvent::OpenConfirmConnection(_) => {
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
    async fn wait_for_target_chain_height_higher_than_consensus_height(
        &self,
        consensus_height: Height,
    ) -> Result<(), Error> {
        let target_chain = self.target_chain().clone();
        let target_chain_latest_height = || target_chain.query_latest_height();

        while consensus_height >= target_chain_latest_height().await? {
            warn!(
                "client consensus proof height too high, \
                 waiting for destination chain to advance beyond {}",
                consensus_height
            );

            thread::sleep(Duration::from_millis(50));
        }

        Ok(())
    }

    async fn validated_expected_connection(
        &self,
        msg_type: ConnectionMsgType,
    ) -> Result<ConnectionEnd, Error> {
        let dst_connection_id = self.target_chain_connection_id().ok_or_else(|| {
            Error::connection_error(ConnectionError::missing_connection_id(
                self.target_chain().id(),
            ))
        })?;

        let prefix = self.source_chain().query_commitment_prefix()?;

        // If there is a connection present on the destination chain, it should look like this:
        let counterparty = Counterparty::new(
            self.source_chain_client_id().clone(),
            self.source_chain_connection_id(),
            prefix,
        );

        // The highest expected state, depends on the message type:
        let highest_state = match msg_type {
            ConnectionMsgType::OpenAck => State::TryOpen,
            ConnectionMsgType::OpenConfirm => State::TryOpen,
            _ => State::Uninitialized,
        };

        let versions = self.source_chain().query_compatible_versions();

        let dst_expected_connection = ConnectionEnd::new(
            highest_state,
            self.target_chain_client_id().clone(),
            counterparty,
            versions,
            ZERO_DURATION,
        );

        // Retrieve existing connection if any
        let (dst_connection, _) = self
            .target_chain()
            .query_connection(&dst_connection_id, QueryHeight::Latest, false)
            .await?;

        // Check if a connection is expected to exist on destination chain
        // A connection must exist on destination chain for Ack and Confirm Tx-es to succeed
        if dst_connection.state_matches(&State::Uninitialized) {
            return Err(Error::connection_error(
                ConnectionError::missing_connection_id(self.target_chain().id()),
            ));
        }

        check_destination_connection_state(
            dst_connection_id.clone(),
            dst_connection,
            dst_expected_connection.clone(),
        )?;

        Ok(dst_expected_connection)
    }
}

impl Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Connection {{ source_connection: {:?}, source_chain: {}, source_client: {}, target_connection: {:?}, target_chain: {}, target_client: {} }}",
            self.side_a.connection_id(),
            self.side_a.chain().id(),
            self.side_a.client_id(),
            self.side_b.connection_id(),
            self.side_b.chain().id(),
            self.side_b.client_id(),
        )
    }
}

/// Verify that the destination connection exhibits the expected state.
fn check_destination_connection_state(
    connection_id: ConnectionId,
    existing_connection: ConnectionEnd,
    expected_connection: ConnectionEnd,
) -> Result<(), Error> {
    let good_client_ids = existing_connection.client_id() == expected_connection.client_id()
        && existing_connection.counterparty().client_id()
            == expected_connection.counterparty().client_id();

    let good_state = *existing_connection.state() as u32 <= *expected_connection.state() as u32;

    let good_connection_ids = existing_connection.counterparty().connection_id().is_none()
        || existing_connection.counterparty().connection_id()
            == expected_connection.counterparty().connection_id();

    let good_version = existing_connection.versions() == expected_connection.versions();

    let good_counterparty_prefix =
        existing_connection.counterparty().prefix() == expected_connection.counterparty().prefix();

    if good_state
        && good_client_ids
        && good_connection_ids
        && good_version
        && good_counterparty_prefix
    {
        Ok(())
    } else {
        Err(Error::connection_error(
            ConnectionError::connection_exists_already(connection_id),
        ))
    }
}

#[cfg(test)]
pub mod connection_tests {
    use std::{str::FromStr, time::Duration};

    use log::info;
    use types::ibc_core::ics24_host::identifier::{ClientId, ConnectionId};

    use crate::{common::QueryHeight, error::Error};

    use super::{Connection, ConnectionSide, CosmosChain};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn connection_open_init_works() {
        init();
        let a_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "/Users/wangert/rust_projects/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let connection = Connection::new(
            ConnectionSide::new(
                cosmos_chain_a,
                ClientId::from_str("07-tendermint-6").unwrap(),
            ),
            ConnectionSide::new(
                cosmos_chain_b,
                ClientId::from_str("07-tendermint-1").unwrap(),
            ),
            Duration::from_secs(100),
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(connection.flipped().build_connection_open_init_and_send());
        match result {
            Ok(events) => println!("Event: {:?}", events),
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    pub fn connection_handshake_works() {
        init();
        let a_file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let mut connection_side_a = ConnectionSide::new(
            cosmos_chain_a,
            ClientId::from_str("07-tendermint-15").unwrap(),
        );
        let mut connection_side_b = ConnectionSide::new(
            cosmos_chain_b,
            ClientId::from_str("07-tendermint-9").unwrap(),
        );

        // connection_side_a.connection_id = Some(ConnectionId::from_str("connection-5").unwrap());
        // connection_side_b.connection_id = Some(ConnectionId::from_str("connection-2").unwrap());
        connection_side_a.connection_id = None;
        connection_side_b.connection_id = None;
        let mut connection =
            Connection::new(connection_side_a, connection_side_b, Duration::from_secs(0));

        let rt = tokio::runtime::Runtime::new().unwrap();
        // let result = rt.block_on(connection.connection_handshake());
        let result = rt.block_on(connection.handshake());
        println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        match result {
            Ok(_) => println!("connection: {:?}", connection),
            Err(e) => println!("{:?}", e),
        }
    }

    #[test]
    pub fn test_error() {
        let e = Error::connection_completed();
        assert_eq!(e.to_string(), e.to_string());
    }
}
