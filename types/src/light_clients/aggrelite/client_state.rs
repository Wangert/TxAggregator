use std::time::Duration;

use ibc_proto::ibc::core::client::v1::Height as CoreClientHeight;
use ibc_proto::ibc::lightclients::tendermint::v1::ClientState as TmClientState;
use ibc_proto::{google::protobuf::Any, Protobuf};
use prost::Message;
use serde::{Deserialize, Serialize};
use tendermint_light_client_verifier::options::Options;

use crate::ibc_core::ics02_client::height::Height;
use crate::light_clients::ics07_tendermint::client_state::AllowUpdate;
use crate::light_clients::ics07_tendermint::trust_level::TrustLevel;
use crate::{
    error::TypesError,
    ibc_core::{ics23_commitment::specs::ProofSpecs, ics24_host::identifier::ChainId},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientState {
    pub chain_id: ChainId,
    pub trust_level: TrustLevel,
    pub trusting_period: Duration,
    pub unbonding_period: Duration,
    pub latest_height: Height,
    pub frozen_height: Option<Height>,
    pub upgrade_path: Vec<String>,
    pub max_clock_drift: Duration,
    pub proof_specs: ProofSpecs,

    // deprecated
    pub allow_update: AllowUpdate,
}

impl ClientState {
    pub fn new(
        chain_id: ChainId,
        trust_level: TrustLevel,
        trusting_period: Duration,
        unbonding_period: Duration,
        max_clock_drift: Duration,
        latest_height: Height,
        proof_specs: ProofSpecs,
        upgrade_path: Vec<String>,
        allow_update: AllowUpdate,
    ) -> Result<ClientState, TypesError> {
        // Basic validation of trusting period and unbonding period: each should be non-zero.
        if trusting_period <= Duration::new(0, 0) {
            return Err(TypesError::invalid_trusting_period(format!(
                "ClientState trusting period ({trusting_period:?}) must be greater than zero"
            )));
        }

        if unbonding_period <= Duration::new(0, 0) {
            return Err(TypesError::invalid_unbonding_period(format!(
                "ClientState unbonding period ({unbonding_period:?}) must be greater than zero"
            )));
        }

        if trusting_period >= unbonding_period {
            return Err(TypesError::invalid_trusting_period(format!(
                "ClientState trusting period ({trusting_period:?}) must be smaller than unbonding period ({unbonding_period:?})",
            )));
        }

        // `TrustThreshold` is guaranteed to be in the range `[0, 1)`,
        // but a zero value is invalid in this context.
        if trust_level.numerator() == 0 {
            return Err(TypesError::invalid_trust_level(
                trust_level.numerator(),
                trust_level.denominator(),
            ));
        }

        // Dividing by zero is undefined so we also rule out a zero denominator.
        // This should be checked already by the `TrustThreshold` constructor
        // but it does not hurt to redo the check here.
        if trust_level.denominator() == 0 {
            return Err(TypesError::invalid_trust_level(
                trust_level.numerator(),
                trust_level.denominator(),
            ));
        }

        // Disallow empty proof-specs
        if proof_specs.is_empty() {
            return Err(TypesError::invalid_proof_specs(
                "ClientState proof specs cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            chain_id,
            trust_level,
            trusting_period,
            unbonding_period,
            max_clock_drift,
            latest_height,
            proof_specs,
            upgrade_path,
            allow_update,
            frozen_height: None,
        })
    }

    pub fn expired(&self, elapsed: Duration) -> bool {
        elapsed > self.trusting_period
    }

    /// Freeze status of the client
    pub fn is_frozen(&self) -> bool {
        self.frozen_height.is_some()
    }

    /// Helper method to produce a [`Options`] struct for use in
    /// Tendermint-specific light client verification.
    pub fn as_light_client_options(&self) -> Options {
        Options {
            trust_threshold: self.trust_level.into(),
            trusting_period: self.trusting_period,
            clock_drift: self.max_clock_drift,
        }
    }
}

impl Protobuf<TmClientState> for ClientState {}

impl TryFrom<TmClientState> for ClientState {
    type Error = TypesError;

    fn try_from(tm_client_state: TmClientState) -> Result<Self, Self::Error> {
        let trust_level_fraction = tm_client_state
            .trust_level
            .ok_or_else(|| TypesError::trust_level("missing".to_string()))?;

        // We need to handle the case where the client is being upgraded and the trust threshold is set to 0/0
        let trust_level =
            if trust_level_fraction.denominator == 0 && trust_level_fraction.numerator == 0 {
                TrustLevel::CLIENT_STATE_RESET
            } else {
                trust_level_fraction.try_into()?
            };

        // In `ClientState`, a `frozen_height` of `0` means "not frozen".
        let frozen_height = tm_client_state
            .frozen_height
            .and_then(|raw_height| raw_height.try_into().ok());

        #[allow(deprecated)]
        Ok(Self {
            chain_id: ChainId::from_string(tm_client_state.chain_id.as_str()),
            trust_level,
            trusting_period: tm_client_state
                .trusting_period
                .ok_or_else(|| TypesError::trusting_period("missing".to_string()))?
                .try_into()
                .map_err(|_| TypesError::trusting_period("invalid".to_string()))?,
            unbonding_period: tm_client_state
                .unbonding_period
                .ok_or_else(|| TypesError::unbonding_period("missing".to_string()))?
                .try_into()
                .map_err(|_| TypesError::unbonding_period("invalid".to_string()))?,
            max_clock_drift: tm_client_state
                .max_clock_drift
                .ok_or_else(|| TypesError::max_clock_drift("missing".to_string()))?
                .try_into()
                .map_err(|_| TypesError::max_clock_drift("invalid".to_string()))?,
            latest_height: tm_client_state
                .latest_height
                .ok_or_else(|| TypesError::latest_height("missing".to_string()))?
                .try_into()
                .map_err(|_| TypesError::latest_height("invalid".to_string()))?,
            frozen_height,
            upgrade_path: tm_client_state.upgrade_path,
            allow_update: AllowUpdate {
                after_expiry: tm_client_state.allow_update_after_expiry,
                after_misbehaviour: tm_client_state.allow_update_after_misbehaviour,
            },
            proof_specs: tm_client_state.proof_specs.into(),
        })
    }
}

impl From<ClientState> for TmClientState {
    fn from(value: ClientState) -> Self {
        #[allow(deprecated)]
        Self {
            chain_id: value.chain_id.to_string(),
            trust_level: Some(value.trust_level.into()),
            trusting_period: Some(value.trusting_period.into()),
            unbonding_period: Some(value.unbonding_period.into()),
            max_clock_drift: Some(value.max_clock_drift.into()),
            frozen_height: Some(value.frozen_height.map(|height| height.into()).unwrap_or(
                CoreClientHeight {
                    revision_number: 0,
                    revision_height: 0,
                },
            )),
            latest_height: Some(value.latest_height.into()),
            proof_specs: value.proof_specs.into(),
            upgrade_path: value.upgrade_path,
            allow_update_after_expiry: value.allow_update.after_expiry,
            allow_update_after_misbehaviour: value.allow_update.after_misbehaviour,
        }
    }
}

impl Protobuf<Any> for ClientState {}

pub const AGGRELITE_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.aggrelite.v1.ClientState";

impl TryFrom<Any> for ClientState {
    type Error = TypesError;

    fn try_from(raw: Any) -> Result<Self, Self::Error> {
        use bytes::Buf;
        use core::ops::Deref;

        fn decode_client_state<B: Buf>(buf: B) -> Result<ClientState, TypesError> {
            TmClientState::decode(buf)
                .map_err(|e| TypesError::tendermint_client_state_decode(e))?
                .try_into()
        }

        match raw.type_url.as_str() {
            AGGRELITE_CLIENT_STATE_TYPE_URL => {
                decode_client_state(raw.value.deref()).map_err(Into::into)
            }
            _ => Err(TypesError::unknown_client_state_type(raw.type_url)),
        }
    }
}

impl From<ClientState> for Any {
    fn from(client_state: ClientState) -> Self {
        Any {
            type_url: AGGRELITE_CLIENT_STATE_TYPE_URL.to_string(),
            value: Protobuf::<TmClientState>::encode_vec(client_state),
        }
    }
}
