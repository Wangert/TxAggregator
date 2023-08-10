use base64::Engine;

use super::error::EncodeError;

pub fn decode_to_string<T: AsRef<[u8]>>(input: T) -> Result<String, EncodeError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|e| EncodeError::base64_decode(e))?;

    String::from_utf8(bytes).map_err(|e| EncodeError::bytes_to_string(e))
}
