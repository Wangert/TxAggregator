use crate::ibc_core::ics02_client::height::Height;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum TimeoutHeight {
    Never,
    At(Height),
}

impl TimeoutHeight {
    pub fn no_timeout() -> Self {
        Self::Never
    }

    /// Revision number to be used in packet commitment computation
    pub fn commitment_revision_number(&self) -> u64 {
        match self {
            Self::At(height) => height.revision_number(),
            Self::Never => 0,
        }
    }

    /// Revision height to be used in packet commitment computation
    pub fn commitment_revision_height(&self) -> u64 {
        match self {
            Self::At(height) => height.revision_height(),
            Self::Never => 0,
        }
    }

    /// Check if a height is *stricly past* the timeout height, and thus is
    /// deemed expired.
    pub fn has_expired(&self, height: Height) -> bool {
        match self {
            Self::At(timeout_height) => height > *timeout_height,
            // When there's no timeout, heights are never expired
            Self::Never => false,
        }
    }

    /// Returns the height formatted for an ABCI event attribute value.
    pub fn to_event_attribute_value(self) -> String {
        match self {
            TimeoutHeight::At(height) => height.to_string(),
            TimeoutHeight::Never => "0-0".into(),
        }
    }
}

impl Default for TimeoutHeight {
    fn default() -> Self {
        Self::Never
    }
}