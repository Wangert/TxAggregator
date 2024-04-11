use std::{cmp::Ordering, fmt::Display};

use ibc_proto::Protobuf;
use serde::{Deserialize, Serialize};
use utils::pretty::PrettySlice;
use ibc_proto::ibc::core::connection::v1::Version as RawVersion;

use super::error::ConnectionError;

/// Stores the identifier and the features supported by a version
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    /// unique version identifier
    identifier: String,
    /// list of features compatible with the specified identifier
    features: Vec<String>,
}

impl Version {
    /// Checks whether or not the given feature is supported in this version
    pub fn is_supported_feature(&self, feature: String) -> bool {
        self.features.contains(&feature)
    }
}

impl Protobuf<RawVersion> for Version {}

impl TryFrom<RawVersion> for Version {
    type Error = ConnectionError;
    fn try_from(value: RawVersion) -> Result<Self, Self::Error> {
        if value.identifier.trim().is_empty() {
            return Err(ConnectionError::empty_versions());
        }
        for feature in value.features.iter() {
            if feature.trim().is_empty() {
                return Err(ConnectionError::empty_features());
            }
        }
        Ok(Version {
            identifier: value.identifier,
            features: value.features,
        })
    }
}

impl From<Version> for RawVersion {
    fn from(value: Version) -> Self {
        Self {
            identifier: value.identifier,
            features: value.features,
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Version {
            identifier: "1".to_string(),
            features: vec![
                "ORDER_ORDERED".to_string(),
                "ORDER_UNORDERED".to_string(),
            ],
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Version {{ identifier: {}, features: {} }}",
            self.identifier,
            PrettySlice(&self.features)
        )
    }
}

/// Returns the lists of supported versions
pub fn get_compatible_versions() -> Vec<Version> {
    vec![Version::default()]
}

/// Selects a version from the intersection of locally supported and counterparty versions.
pub fn pick_version(
    supported_versions: Vec<Version>,
    counterparty_versions: Vec<Version>,
) -> Result<Version, ConnectionError> {
    let mut intersection: Vec<Version> = Vec::new();
    for s in supported_versions.iter() {
        for c in counterparty_versions.iter() {
            if c.identifier != s.identifier {
                continue;
            }
            for feature in c.features.iter() {
                if feature.trim().is_empty() {
                    return Err(ConnectionError::empty_features());
                }
            }
            intersection.append(&mut vec![s.clone()]);
        }
    }
    intersection.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    if intersection.is_empty() {
        return Err(ConnectionError::no_common_version());
    }
    Ok(intersection[0].clone())
}