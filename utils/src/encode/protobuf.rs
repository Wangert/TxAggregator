use prost::Message;

use super::error::EncodeError;

pub fn encode_to_bytes<O>(object: &O) -> Result<Vec<u8>, EncodeError> 
where
    O: Sized + Message,
{
    let mut object_bytes = vec![];
    prost::Message::encode(object, &mut object_bytes)
        .map_err(|e| EncodeError::protobuf_encode(e))?;

    Ok(object_bytes)
}