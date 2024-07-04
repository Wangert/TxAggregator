use std::{
    fmt::{Display, Error as FmtError, Formatter},
    thread,
    time::Duration,
};

use futures::TryFutureExt;
use ibc_proto::google::protobuf::Any;
use serde::Serialize;
use tracing::{debug, info, trace, warn};
use types::{
    ibc_core::{
        ics02_client::{header::AnyHeader, height::Height, update_client::MsgUpdateClient},
        ics04_channel::{
            channel::{
                check_target_channel_state, ChannelEnd, ChannelMsgType, Counterparty, Ordering,
                State,
            },
            error::ChannelError,
            events::extract_channel_id,
            message::{
                MsgChannelOpenAck, MsgChannelOpenConfirm, MsgChannelOpenInit, MsgChannelOpenTry,
            },
            version::Version,
        },
        ics24_host::identifier::{
            ChainId, ChannelId, ClientId, ConnectionId, PortId, AGGRELITE_CLIENT_PREFIX,
            TENDERMINT_CLIENT_PREFIX,
        },
    },
    ibc_events::IbcEvent,
    light_clients::{
        header_type::AdjustHeadersType, ics07_tendermint::header::TENDERMINT_HEADER_TYPE_URL,
    },
    message::Msg,
};

use crate::{chain::CosmosChain, common::QueryHeight, connection::Connection, error::Error};

#[derive(Clone, Debug, Serialize)]
pub struct ChannelSide {
    #[serde(skip)]
    pub chain: CosmosChain,
    client_id: ClientId,
    connection_id: ConnectionId,
    port_id: PortId,
    channel_id: Option<ChannelId>,
    version: Option<Version>,
}

impl Display for ChannelSide {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        match (&self.channel_id, &self.version) {
            (Some(channel_id), Some(version)) => write!(f, "ChannelSide {{ chain: {:?}, client_id: {}, connection_id: {}, port_id: {}, channel_id: {}, version: {} }}", self.chain, self.client_id, self.connection_id, self.port_id, channel_id, version),
            (Some(channel_id), None) => write!(f, "ChannelSide {{ chain: {:?}, client_id: {}, connection_id: {}, port_id: {}, channel_id: {}, version: None }}", self.chain, self.client_id, self.connection_id, self.port_id, channel_id),
            (None, Some(version)) => write!(f, "ChannelSide {{ chain: {:?}, client_id: {}, connection_id: {}, port_id: {}, channel_id: None, version: {} }}", self.chain, self.client_id, self.connection_id, self.port_id, version),
            (None, None) => write!(f, "ChannelSide {{ chain: {:?}, client_id: {}, connection_id: {}, port_id: {}, channel_id: None, version: None }}", self.chain, self.client_id, self.connection_id, self.port_id),
        }
    }
}

impl ChannelSide {
    pub fn new(
        chain: CosmosChain,
        client_id: ClientId,
        connection_id: ConnectionId,
        port_id: PortId,
        channel_id: Option<ChannelId>,
        version: Option<Version>,
    ) -> ChannelSide {
        Self {
            chain,
            client_id,
            connection_id,
            port_id,
            channel_id,
            version,
        }
    }

    pub fn chain(&self) -> &CosmosChain {
        &self.chain
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain.id()
    }

    pub fn client_id(&self) -> &ClientId {
        &self.client_id
    }

    pub fn connection_id(&self) -> &ConnectionId {
        &self.connection_id
    }

    pub fn port_id(&self) -> &PortId {
        &self.port_id
    }

    pub fn channel_id(&self) -> Option<&ChannelId> {
        self.channel_id.as_ref()
    }

    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    pub fn map_chain(self, mapper: impl Fn(CosmosChain) -> CosmosChain) -> ChannelSide {
        ChannelSide {
            chain: mapper(self.chain),
            client_id: self.client_id,
            connection_id: self.connection_id,
            port_id: self.port_id,
            channel_id: self.channel_id,
            version: self.version,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Channel {
    pub ordering: Ordering,
    pub side_a: ChannelSide,
    pub side_b: ChannelSide,
    pub connection_delay: Duration,
}

// impl Display for Channel {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
//         write!(
//             f,
//             "Channel {{ ordering: {}, a_side: {}, b_side: {}, connection_delay: {:?} }}",
//             self.ordering, self.side_a, self.side_b, &self.connection_delay
//         )
//     }
// }

impl Channel {
    pub fn new(
        connection: Connection,
        ordering: Ordering,
        a_port: PortId,
        b_port: PortId,
        version: Option<Version>,
    ) -> Result<Self, Error> {
        let side_a = ChannelSide {
            chain: connection.source_chain().clone(),
            client_id: connection.source_chain_client_id(),
            connection_id: connection
                .source_chain_connection_id()
                .ok_or_else(Error::empty_connection_id)?,
            port_id: a_port,
            channel_id: None,
            version: version.clone(),
        };

        let side_b = ChannelSide {
            chain: connection.target_chain().clone(),
            client_id: connection.target_chain_client_id(),
            connection_id: connection
                .target_chain_connection_id()
                .ok_or_else(Error::empty_connection_id)?,
            port_id: b_port,
            channel_id: None,
            version,
        };

        Ok(Self {
            ordering,
            side_a,
            side_b,
            connection_delay: connection.delay_period,
        })
    }

    pub fn source_chain(&self) -> &CosmosChain {
        self.side_a.chain()
    }

    pub fn target_chain(&self) -> &CosmosChain {
        self.side_b.chain()
    }

    pub fn source_chain_client_id(&self) -> &ClientId {
        self.side_a.client_id()
    }

    pub fn target_chain_client_id(&self) -> &ClientId {
        self.side_b.client_id()
    }

    pub fn source_chain_connection_id(&self) -> &ConnectionId {
        self.side_a.connection_id()
    }

    pub fn target_chain_connection_id(&self) -> &ConnectionId {
        self.side_b.connection_id()
    }

    pub fn source_chain_port_id(&self) -> &PortId {
        self.side_a.port_id()
    }

    pub fn target_chain_port_id(&self) -> &PortId {
        self.side_b.port_id()
    }

    pub fn source_chain_channel_id(&self) -> Option<&ChannelId> {
        self.side_a.channel_id()
    }

    pub fn target_chain_channel_id(&self) -> Option<&ChannelId> {
        self.side_b.channel_id()
    }

    pub fn source_chain_version(&self) -> Option<&Version> {
        self.side_a.version()
    }

    pub fn target_chain_version(&self) -> Option<&Version> {
        self.side_b.version()
    }

    fn update_channel_id(
        &mut self,
        connection_id: ConnectionId,
        channel_id: Option<ChannelId>,
    ) -> Result<(), Error> {
        if self.source_chain_connection_id().clone() == connection_id {
            self.side_a.channel_id = channel_id;
        } else if self.target_chain_connection_id().clone() == connection_id {
            self.side_b.channel_id = channel_id;
        } else {
            return Err(Error::channel_handshke_abnormal());
        }

        Ok(())
    }

    pub async fn handshake(&mut self) -> Result<(), Error> {
        loop {
            let event_result = self.channel_handshake().await;
            match event_result {
                Ok(Some(IbcEvent::OpenInitChannel(e))) => {
                    let init_chain_connection_id = e.connection_id.clone();
                    let update_channel_id = e.channel_id().cloned();
                    self.update_channel_id(init_chain_connection_id, update_channel_id)?;
                }
                Ok(Some(IbcEvent::OpenTryChannel(e))) => {
                    let try_chain_connection_id = e.connection_id.clone();
                    let update_channel_id = e.channel_id().cloned();
                    self.update_channel_id(try_chain_connection_id, update_channel_id)?;
                }
                Err(e) if e.to_string() == Error::channel_completed().to_string() => {
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

    /// Sends a channel open handshake message.
    pub async fn channel_handshake(&mut self) -> Result<Option<IbcEvent>, Error> {
        let (a_state, b_state) = self.update_channel_states().await?;
        debug!(
            "do_chan_open_handshake with channel end states: {}, {}",
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
            // send the Init message to chain a (source)
            (State::Uninitialized, State::Uninitialized) => {
                let event = self.flipped().channel_open_init().await?;
                let channel_id = extract_channel_id(&event).map_err(Error::channel_error)?;
                self.side_a.channel_id = Some(channel_id.clone());

                ibc_event = Some(event);
            }

            // send the Try message to chain a (source)
            (State::Uninitialized, State::Init) | (State::Init, State::Init) => {
                let event = self.flipped().channel_open_try().await?;

                let channel_id = extract_channel_id(&event).map_err(Error::channel_error)?;
                self.side_a.channel_id = Some(channel_id.clone());

                ibc_event = Some(event);
            }

            // send the Try message to chain b (target)
            (State::Init, State::Uninitialized) => {
                let event = self.channel_open_try().await?;

                let channel_id = extract_channel_id(&event).map_err(Error::channel_error)?;
                self.side_b.channel_id = Some(channel_id.clone());

                ibc_event = Some(event);
            }

            // send the Ack message to chain a (source)
            (State::Init, State::TryOpen) | (State::TryOpen, State::TryOpen) => {
                let event = self.flipped().channel_open_ack().await?;

                ibc_event = Some(event);
            }

            // send the Ack message to chain b (target)
            (State::TryOpen, State::Init) => {
                let event = self.channel_open_ack().await?;
                ibc_event = Some(event);
            }

            // send the Confirm message to chain b (target)
            (State::Open, State::TryOpen) => {
                let event = self.channel_open_confirm().await?;
                ibc_event = Some(event);
            }

            // send the Confirm message to chain a (source)
            (State::TryOpen, State::Open) => {
                let event = self.flipped().channel_open_confirm().await?;
                ibc_event = Some(event);
            }

            (State::Open, State::Open) => {
                info!("channel handshake already finished for {}", self);
                println!("channel handshake already finished for {}", self);
                return Err(Error::channel_completed());
            }

            (a_state, b_state) => {
                warn!(
                    "do_conn_open_handshake does not handle channel end state combination: \
                    {}-{}, {}-{}. will retry to account for RPC node data availability issues.",
                    self.source_chain().id(),
                    a_state,
                    self.target_chain().id(),
                    b_state
                );
            }
        }

        Ok(ibc_event)
    }

    async fn update_channel_states(&mut self) -> Result<(State, State), Error> {
        let old_chan_a_id = self.source_chain_channel_id().cloned();
        let chan_a_port_id = self.source_chain_port_id().clone();
        let old_chan_b_id = self.target_chain_channel_id().cloned();

        let a_channel = if let Some(chan_id) = old_chan_a_id.as_ref() {
            let (a_channel, _) = self
                .source_chain()
                .query_channel(chan_id, &chan_a_port_id, QueryHeight::Latest, true)
                .await?;
            a_channel
        } else {
            ChannelEnd::default()
        };
        // let a_channel = self.a_channel(old_chan_a_id)?;
        let a_counterparty_id = a_channel.counterparty().channel_id();

        if a_counterparty_id.is_some() && a_counterparty_id != old_chan_b_id.as_ref() {
            self.side_b.channel_id = a_counterparty_id.cloned();
        }

        let updated_chan_b_id = self.target_chain_channel_id();

        let b_channel = if let Some(chan_id) = updated_chan_b_id.as_ref() {
            let (b_channel, _) = self
                .target_chain()
                .query_channel(chan_id, &chan_a_port_id, QueryHeight::Latest, true)
                .await?;
            b_channel
        } else {
            ChannelEnd::default()
        };

        // let b_channel = self.b_channel(updated_relayer_b_id)?;
        let b_counterparty_id = b_channel.counterparty().channel_id();

        if b_counterparty_id.is_some() && b_counterparty_id != old_chan_a_id.as_ref() {
            if updated_chan_b_id == old_chan_b_id.as_ref() {
                self.side_a.channel_id = b_counterparty_id.cloned();
            } else {
                panic!(
                    "mismatched channel ids in channel ends: {} - {} and {} - {}",
                    self.source_chain().id(),
                    a_channel,
                    self.target_chain().id(),
                    b_channel,
                );
            }
        }
        Ok((*a_channel.state(), *b_channel.state()))
    }

    pub async fn channel_open_init(&self) -> Result<IbcEvent, Error> {
        let msgs = self.build_channel_open_init()?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(msgs)
            .await?;

        // Find the relevant event for channel open init
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenInitChannel(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_init_event()))?;

        match &result.event {
            IbcEvent::OpenInitChannel(_) => {
                info!("🎊  {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    pub async fn channel_open_try(&self) -> Result<IbcEvent, Error> {
        let msgs = self.build_channel_open_try().await?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(msgs)
            .await?;

        // Find the relevant event for channel open try
        let result = events
            .into_iter()
            .find(|events_with_height| {
                matches!(events_with_height.event, IbcEvent::OpenTryChannel(_))
                    || matches!(events_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_try_event()))?;

        match &result.event {
            IbcEvent::OpenTryChannel(_) => {
                info!("🎊  {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    pub async fn channel_open_ack(&self) -> Result<IbcEvent, Error> {
        let msgs = self.build_chan_open_ack().await?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(msgs)
            .await?;

        // Find the relevant event for channel open ack
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenAckChannel(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_ack_event()))?;

        match &result.event {
            IbcEvent::OpenAckChannel(_) => {
                info!("🎊  {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    pub async fn channel_open_confirm(&self) -> Result<IbcEvent, Error> {
        let msgs = self.build_chan_open_confirm().await?;

        let events = self
            .target_chain()
            .send_messages_and_wait_commit(msgs)
            .await?;

        // Find the relevant event for channel open confirm
        let result = events
            .into_iter()
            .find(|event_with_height| {
                matches!(event_with_height.event, IbcEvent::OpenConfirmChannel(_))
                    || matches!(event_with_height.event, IbcEvent::CosmosChainError(_))
            })
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_confirm_event()))?;

        match &result.event {
            IbcEvent::OpenConfirmChannel(_) => {
                info!("🎊  {} => {}", self.target_chain().id(), result);
                Ok(result.event)
            }
            IbcEvent::CosmosChainError(e) => Err(Error::tx_response(e.clone())),
            _ => Err(Error::invalid_event(result.event)),
        }
    }

    pub fn build_channel_open_init(&self) -> Result<Vec<Any>, Error> {
        let signer = self.target_chain().account().get_signer()?;

        let counterparty = Counterparty::new(self.source_chain_port_id().clone(), None);

        let version = self
            .target_chain_version()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_version()))?
            .clone();

        let channel_end = ChannelEnd::new(
            State::Init,
            self.ordering,
            counterparty,
            vec![self.target_chain_connection_id().clone()],
            version,
            0,
        );

        // Build the channel OpenInit message
        let new_msg = MsgChannelOpenInit {
            port_id: self.target_chain_port_id().clone(),
            channel: channel_end,
            signer,
        };

        Ok(vec![new_msg.to_any()])
    }

    pub async fn build_channel_open_try(&self) -> Result<Vec<Any>, Error> {
        // Source channel ID must be specified
        let source_channel_id = self
            .source_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;

        // Channel must exist on source chain
        let (source_channel_end, _) = self
            .source_chain()
            .query_channel(
                source_channel_id,
                self.source_chain_port_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        if source_channel_end.counterparty().port_id() != self.target_chain_port_id() {
            return Err(Error::channel_error(ChannelError::mismatch_port(
                "Channel OpenTry".to_string(),
            )));
        }

        // Connection must exist on target chain
        self.target_chain()
            .query_connection(
                self.target_chain_connection_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;

        let proofs = self
            .source_chain()
            .build_channel_proofs(self.source_chain_port_id(), source_channel_id, query_height)
            .await?;

        // Build message(s) to update client on target chain
        let target_update_client_msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        let update_event = self
            .target_chain()
            .send_messages_and_wait_commit(target_update_client_msgs)
            .await?;

        println!("update client: {:?}", update_event);

        let counterparty = Counterparty::new(
            self.source_chain_port_id().clone(),
            self.source_chain_channel_id().cloned(),
        );

        // Reuse the version that was either set on ChanOpenInit or overwritten by the application.
        let version = source_channel_end.version().clone();

        let channel = ChannelEnd::new(
            State::TryOpen,
            *source_channel_end.ordering(),
            counterparty,
            vec![self.target_chain_connection_id().clone()],
            version,
            0,
        );

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        let previous_channel_id = if source_channel_end.counterparty().channel_id.is_none() {
            self.target_chain_channel_id().cloned()
        } else {
            source_channel_end.counterparty().channel_id.clone()
        };

        // Build the domain type message
        let new_msg = MsgChannelOpenTry {
            port_id: self.target_chain_port_id().clone(),
            previous_channel_id: None,
            counterparty_version: source_channel_end.version().clone(),
            channel,
            proofs,
            signer,
        };

        let mut msgs = vec![];

        msgs.push(new_msg.to_any());
        Ok(msgs)
    }

    pub async fn build_chan_open_ack(&self) -> Result<Vec<Any>, Error> {
        // Source and destination channel IDs must be specified
        let source_channel_id = self
            .source_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;
        let target_channel_id = self
            .target_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;

        // Check that the destination chain will accept the Ack message
        self.validated_expected_channel(ChannelMsgType::OpenAck)
            .await?;

        // Channel must exist on source
        let (src_channel, _) = self
            .source_chain()
            .query_channel(
                source_channel_id,
                self.source_chain_port_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        // Connection must exist on target chain
        self.target_chain()
            .query_connection(
                self.target_chain_connection_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;

        let proofs = self
            .source_chain()
            .build_channel_proofs(self.source_chain_port_id(), source_channel_id, query_height)
            .await?;

        // Build message(s) to update client on target chain
        let mut msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        // Build the domain type message
        let new_msg = MsgChannelOpenAck {
            port_id: self.target_chain_port_id().clone(),
            channel_id: target_channel_id.clone(),
            counterparty_channel_id: source_channel_id.clone(),
            counterparty_version: src_channel.version().clone(),
            proofs,
            signer,
        };

        msgs.push(new_msg.to_any());
        Ok(msgs)
    }

    pub async fn build_chan_open_confirm(&self) -> Result<Vec<Any>, Error> {
        // Source and target channel IDs must be specified
        let source_channel_id = self
            .source_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;
        let target_channel_id = self
            .target_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;

        // Check that the target chain will accept the message
        self.validated_expected_channel(ChannelMsgType::OpenConfirm)
            .await?;

        // Channel must exist on source
        self.source_chain()
            .query_channel(
                source_channel_id,
                self.source_chain_port_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        // Connection must exist on target chain
        self.target_chain()
            .query_connection(
                self.target_chain_connection_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        let query_height = self.source_chain().query_latest_height().await?;

        let proofs = self
            .source_chain()
            .build_channel_proofs(self.source_chain_port_id(), source_channel_id, query_height)
            .await?;

        // Build message to update client on target chain
        let mut msgs = self
            .build_update_client_on_target_chain(proofs.height())
            .await?;

        // Get signer
        let signer = self.target_chain().account().get_signer()?;

        // Build the domain type message
        let new_msg = MsgChannelOpenConfirm {
            port_id: self.target_chain_port_id().clone(),
            channel_id: target_channel_id.clone(),
            proofs,
            signer,
        };

        msgs.push(new_msg.to_any());
        Ok(msgs)
    }

    pub fn flipped(&self) -> Channel {
        Channel {
            ordering: self.ordering,
            side_a: self.side_b.clone(),
            side_b: self.side_a.clone(),
            connection_delay: self.connection_delay,
        }
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
            self.target_chain()
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
            self.target_chain()
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

        if client_id.check_type(TENDERMINT_HEADER_TYPE_URL) {}
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

    async fn validated_expected_channel(
        &self,
        msg_type: ChannelMsgType,
    ) -> Result<ChannelEnd, Error> {
        // Target channel ID must be specified
        let target_channel_id = self
            .target_chain_channel_id()
            .ok_or_else(|| Error::channel_error(ChannelError::missing_channel_id()))?;

        let counterparty = Counterparty::new(
            self.source_chain_port_id().clone(),
            self.source_chain_channel_id().cloned(),
        );

        // The highest expected state, depends on the message type:
        let highest_state = match msg_type {
            ChannelMsgType::OpenAck => State::TryOpen,
            ChannelMsgType::OpenConfirm => State::TryOpen,
            ChannelMsgType::CloseConfirm => State::Open,
            _ => State::Uninitialized,
        };

        let dst_expected_channel = ChannelEnd::new(
            highest_state,
            self.ordering,
            counterparty,
            vec![self.target_chain_connection_id().clone()],
            Version::empty(),
            0,
        );

        // Retrieve existing channel
        let (dst_channel, _) = self
            .target_chain()
            .query_channel(
                target_channel_id,
                self.target_chain_port_id(),
                QueryHeight::Latest,
                false,
            )
            .await?;

        // Check if a channel is expected to exist on target chain
        if dst_channel.state_matches(&State::Uninitialized) {
            return Err(Error::channel_error(
                ChannelError::missing_channel_on_target(),
            ));
        }

        check_target_channel_state(target_channel_id, &dst_channel, &dst_expected_channel)
            .map_err(Error::channel_error)?;

        Ok(dst_expected_channel)
    }
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Source Chain:{{ chain_id: {}, channel_id: {:?}, client_id: {}, connection_id: {:?}, port_id: {}, version: {:?} }}\nTarget Chain:{{ chain_id: {}, channel_id: {:?}, client_id: {}, connection_id: {:?}, port_id: {}, version: {:?} }}",
            self.side_a.chain().id(),
            self.side_a.channel_id(),
            self.side_a.client_id(),
            self.side_a.connection_id(),
            self.side_a.port_id(),
            self.side_a.version(),
            self.side_b.chain().id(),
            self.side_b.channel_id(),
            self.side_b.client_id(),
            self.side_b.connection_id(),
            self.side_b.port_id(),
            self.side_b.version()
        )
    }
}

#[cfg(test)]
pub mod channel_tests {
    use std::{str::FromStr, time::Duration};

    use types::ibc_core::{
        ics04_channel::{channel::Ordering, version::Version},
        ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId},
    };

    use crate::chain::CosmosChain;

    use super::{Channel, ChannelSide};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    pub fn channel_handshake_works() {
        init();
        let a_file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_a_config.toml";
        let b_file_path =
            "C:/Users/admin/Documents/GitHub/TxAggregator/cosmos_chain/src/config/chain_b_config.toml";

        let cosmos_chain_a = CosmosChain::new(a_file_path);
        let cosmos_chain_b = CosmosChain::new(b_file_path);

        let channel_side_a = ChannelSide {
            chain: cosmos_chain_a,
            client_id: ClientId::from_str("07-tendermint-14").unwrap(),
            connection_id: ConnectionId::from_str("connection-6").unwrap(),
            port_id: PortId::from_str("blog").unwrap(),
            channel_id: None,
            version: Some(Version("blog-1".to_string())),
        };

        let channel_side_b = ChannelSide {
            chain: cosmos_chain_b,
            client_id: ClientId::from_str("07-tendermint-7").unwrap(),
            connection_id: ConnectionId::from_str("connection-4").unwrap(),
            port_id: PortId::from_str("blog").unwrap(),
            channel_id: None,
            version: Some(Version("blog-1".to_string())),
        };

        // let channel_side_a = ChannelSide {
        //     chain: cosmos_chain_a,
        //     client_id: ClientId::from_str("07-tendermint-10").unwrap(),
        //     connection_id: ConnectionId::from_str("connection-5").unwrap(),
        //     port_id: PortId::from_str("transfer").unwrap(),
        //     channel_id: Some(ChannelId::from_str("channel-2").unwrap()),
        //     version: Some(Version::default()),
        // };

        // let channel_side_b = ChannelSide {
        //     chain: cosmos_chain_b,
        //     client_id: ClientId::from_str("07-tendermint-4").unwrap(),
        //     connection_id: ConnectionId::from_str("connection-2").unwrap(),
        //     port_id: PortId::from_str("transfer").unwrap(),
        //     channel_id: Some(ChannelId::from_str("channel-0").unwrap()),
        //     version: Some(Version::default()),
        // };

        let mut channel = Channel {
            ordering: Ordering::Unordered,
            side_a: channel_side_a,
            side_b: channel_side_b,
            connection_delay: Duration::from_secs(100),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        // let result = rt.block_on(channel.channel_handshake());
        let result = rt.block_on(channel.handshake());
        println!("$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$");
        match result {
            Ok(events) => println!("Event: {:?}", events),
            Err(e) => println!("{:?}", e),
        }
    }
}
