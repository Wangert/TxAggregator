use std::fmt::{Display, Formatter, Error};

use ibc_proto::{Protobuf, ibc::lightclients::tendermint::v1::Fraction};
use num_rational::Ratio;
use serde::{Serialize, Deserialize};
use tendermint::trust_threshold::TrustThresholdFraction;

use crate::error::TypesError;

#[derive(Debug, Clone, Copy)]
pub struct TrustLevel(Ratio<u64>);

impl TrustLevel {
    /// Constant for a trust threshold of 1/3.
    pub const ONE_THIRD: Self = Self(Ratio::new_raw(1, 3));

    /// Constant for a trust threshold of 2/3.
    pub const TWO_THIRDS: Self = Self(Ratio::new_raw(2, 3));

    /// Constant for a trust threshold of 0/0.
    ///
    /// IMPORTANT: Only to be used for resetting the client state
    /// during a client upgrade. Using this value anywhere else
    /// might lead to panics.
    pub const CLIENT_STATE_RESET: Self = Self(Ratio::new_raw(0, 0));

    /// Instantiate a TrustThreshold with the given denominator and
    /// numerator.
    ///
    /// The constructor succeeds as long as the resulting fraction
    /// is a rational number in the range`[0, 1)`.
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, TypesError> {
        // The fraction cannot be bigger than 1, nor can the denominator be zero
        if numerator > denominator || denominator == 0 {
            return Err(TypesError::invalid_trust_level(numerator, denominator));
        }

        Ok(Self(Ratio::new(numerator, denominator)))
    }

    /// The numerator of the fraction underlying this trust threshold.
    pub fn numerator(&self) -> u64 {
        *self.0.numer()
    }

    /// The denominator of the fraction underlying this trust threshold.
    pub fn denominator(&self) -> u64 {
        *self.0.denom()
    }
}

/// Conversion from Tendermint domain type into IBC domain type.
impl From<TrustThresholdFraction> for TrustLevel {
    fn from(ttf: TrustThresholdFraction) -> Self {
        Self(Ratio::new_raw(ttf.numerator(), ttf.denominator()))
    }
}

/// Conversion from IBC domain type into Tendermint domain type.
impl From<TrustLevel> for TrustThresholdFraction {
    fn from(tl: TrustLevel) -> TrustThresholdFraction {
        TrustThresholdFraction::new(tl.numerator(), tl.denominator())
            .expect("trust threshold should have been valid")
    }
}

impl Protobuf<Fraction> for TrustLevel {}

impl From<TrustLevel> for Fraction {
    fn from(tl: TrustLevel) -> Self {
        Fraction {
            numerator: tl.numerator(),
            denominator: tl.denominator(),
        }
    }
}

impl TryFrom<Fraction> for TrustLevel {
    type Error = TypesError;

    fn try_from(value: Fraction) -> Result<Self, Self::Error> {
        Self::new(value.numerator, value.denominator)
    }
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::TWO_THIRDS
    }
}

impl Display for TrustLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}/{}", self.numerator(), self.denominator())
    }
}

impl Serialize for TrustLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct TrustThreshold {
            numerator: u64,
            denominator: u64,
        }

        let tt = TrustThreshold {
            numerator: self.numerator(),
            denominator: self.denominator(),
        };

        tt.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TrustLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TrustThreshold {
            numerator: u64,
            denominator: u64,
        }

        let tt = TrustThreshold::deserialize(deserializer)?;
        Self::new(tt.numerator, tt.denominator).map_err(serde::de::Error::custom)
    }
}