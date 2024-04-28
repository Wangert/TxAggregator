use base64::Engine;
use serde::Serialize;

use super::error::EncodeError;

pub fn decode_to_string<T: AsRef<[u8]>>(input: T) -> Result<String, EncodeError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| EncodeError::base64_decode(e))?;

    String::from_utf8(bytes).map_err(|e| EncodeError::bytes_to_string(e))
}

pub fn encode_to_base64_string<T>(value: &T) -> Result<String, EncodeError>
where
    T: ?Sized + Serialize,
{
    let json_str = serde_json::to_string(value).map_err(EncodeError::serde_json_error)?;
    let base64_str = base64::engine::general_purpose::STANDARD.encode(json_str);

    Ok(base64_str)
}