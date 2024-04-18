use std::{convert::Infallible, fmt::{Display, Error as FmtError, Formatter}, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Version(pub String);

impl Version {
    pub fn new(v: String) -> Self {
        Self(v)
    }

    // pub fn ics20() -> Self {
    //     Self::new(transfer::VERSION.to_string())
    // }

    // pub fn ics20_with_fee() -> Self {
    //     let val = json::json!({
    //         "fee_version": "ics29-1",
    //         "app_version": transfer::VERSION,
    //     });

    //     Self::new(val.to_string())
    // }

    pub fn empty() -> Self {
        Self::new("".to_string())
    }

    pub fn supports_fee(&self) -> bool {
        serde_json::from_str::<serde_json::Value>(&self.0)
            .ok()
            .and_then(|val| {
                let _app_version = val.get("app_version")?.as_str()?;

                let fee_version = val.get("fee_version")?.as_str()?;

                Some(fee_version == "ics29-1")
            })
            .unwrap_or(false)
    }
}

impl From<String> for Version {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl FromStr for Version {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

/// The default version is empty (unspecified).
impl Default for Version {
    fn default() -> Self {
        Version::empty()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}