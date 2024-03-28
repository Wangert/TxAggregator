use core::fmt::Debug;
use ibc_proto::{google::protobuf::Any, Protobuf};
use serde::{Deserialize, Serialize};

use crate::{error::TypesError, ibc_core::ics24_host::identifier::ClientType, light_clients::ics07_tendermint::header::TENDERMINT_HEADER_TYPE_URL, timestamp::Timestamp};

use super::height::Height;
use crate::light_clients::ics07_tendermint::header::{ decode_header as tm_decode_header, Header as TmHeader};

/// Abstract of consensus state update information
pub trait Header: Debug + Send + Sync // Any: From<Self>,
{
    /// The type of client (eg. Tendermint)
    fn client_type(&self) -> ClientType;

    /// The height of the consensus state
    fn height(&self) -> Height;

    /// The timestamp of the consensus state
    fn timestamp(&self) -> Timestamp;
}

/// Decodes an encoded header into a known `Header` type,
pub fn decode_header(header_bytes: &[u8]) -> Result<AnyHeader, TypesError> {
    // For now, we only have tendermint; however when there is more than one, we
    // can try decoding into all the known types, and return an error only if
    // none work
    let header: TmHeader =
        Protobuf::<Any>::decode(header_bytes).map_err(TypesError::invalid_raw_header)?;

    Ok(AnyHeader::Tendermint(header))
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum AnyHeader {
    Tendermint(TmHeader),
}

impl Header for AnyHeader {
    fn client_type(&self) -> ClientType {
        match self {
            Self::Tendermint(header) => header.client_type(),
        }
    }

    fn height(&self) -> Height {
        match self {
            Self::Tendermint(header) => header.height(),
        }
    }

    fn timestamp(&self) -> Timestamp {
        match self {
            Self::Tendermint(header) => header.timestamp(),
        }
    }
}

impl Protobuf<Any> for AnyHeader {}

impl TryFrom<Any> for AnyHeader {
    type Error = TypesError;

    fn try_from(raw: Any) -> Result<Self, TypesError> {
        match raw.type_url.as_str() {
            TENDERMINT_HEADER_TYPE_URL => {
                let val = tm_decode_header(raw.value.as_slice())?;
                Ok(AnyHeader::Tendermint(val))
            }

            _ => Err(TypesError::unknown_header_type(raw.type_url)),
        }
    }
}

impl From<AnyHeader> for Any {
    fn from(value: AnyHeader) -> Self {
        use ibc_proto::ibc::lightclients::tendermint::v1::Header as RawHeader;

        match value {
            AnyHeader::Tendermint(header) => Any {
                type_url: TENDERMINT_HEADER_TYPE_URL.to_string(),
                value: Protobuf::<RawHeader>::encode_vec(header),
            },
        }
    }
}

impl From<TmHeader> for AnyHeader {
    fn from(header: TmHeader) -> Self {
        Self::Tendermint(header)
    }
}
