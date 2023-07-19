use bech32::{ToBase32, FromBase32};

use super::error::EncodeError;

pub fn encode(hrp: &str, data: &[u8]) -> Result<String, EncodeError> {
    bech32::encode(hrp, data.to_base32(), bech32::Variant::Bech32).map_err(|e| EncodeError::bech32_encode(e))
}

pub fn decode(data: &str) -> Result<Vec<u8>, EncodeError> {
    let (_, data, _) = bech32::decode(data).map_err(|e| EncodeError::bech32_decode(e))?;
    Vec::from_base32(&data).map_err(|e| EncodeError::bech32_decode(e))
}
